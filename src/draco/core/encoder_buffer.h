// Copyright 2016 The Draco Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
#ifndef DRACO_CORE_ENCODER_BUFFER_H_
#define DRACO_CORE_ENCODER_BUFFER_H_

#include <memory>
#include <vector>

#include "draco/core/bit_utils.h"
#include "draco/core/macros.h"

#ifdef DRACO_RUST_CORE
// Include Rust C API header when Rust support is enabled
extern "C" {
#include "draco_core.h"
}
#endif

namespace draco {

// Class representing a buffer that can be used for either for byte-aligned
// encoding of arbitrary data structures or for encoding of variable-length
// bit data.
class EncoderBuffer {
 public:
#ifdef DRACO_RUST_CORE
  // Constructor that uses Rust implementation
  EncoderBuffer() : rust_buffer_(draco_encoder_buffer_create()) {}

  // Destructor
  ~EncoderBuffer() {
    if (rust_buffer_) {
      draco_encoder_buffer_destroy(rust_buffer_);
    }
  }

  // Copy constructor
  EncoderBuffer(const EncoderBuffer &other) : rust_buffer_(draco_encoder_buffer_create()) {
    // Copy data from other buffer
    if (other.rust_buffer_) {
      const uint8_t* data = draco_encoder_buffer_data(other.rust_buffer_);
      size_t size = draco_encoder_buffer_size(other.rust_buffer_);
      draco_encoder_buffer_encode(rust_buffer_, data, size);
    }
  }

  // Assignment operator
  EncoderBuffer& operator=(const EncoderBuffer &other) {
    if (this != &other) {
      Clear();
      if (other.rust_buffer_) {
        const uint8_t* data = draco_encoder_buffer_data(other.rust_buffer_);
        size_t size = draco_encoder_buffer_size(other.rust_buffer_);
        draco_encoder_buffer_encode(rust_buffer_, data, size);
      }
    }
    return *this;
  }

  void Clear() {
    draco_encoder_buffer_clear(rust_buffer_);
  }

  void Resize(int64_t nbytes) {
    // Note: Rust implementation doesn't have direct resize, but we can recreate
    if (nbytes < 0) return;
    // For now, we clear and the buffer will grow as needed
    Clear();
  }

  bool StartBitEncoding(int64_t required_bits, bool encode_size) {
    if (required_bits < 0) return false;
    return draco_encoder_buffer_start_bit_encoding(rust_buffer_,
                                                   static_cast<size_t>(required_bits),
                                                   encode_size) == DRACO_STATUS_OK;
  }

  void EndBitEncoding() {
    draco_encoder_buffer_end_bit_encoding(rust_buffer_);
  }

  bool EncodeLeastSignificantBits32(int nbits, uint32_t value) {
    return draco_encoder_buffer_encode_bits(rust_buffer_,
                                           static_cast<uint32_t>(nbits),
                                           value) == DRACO_STATUS_OK;
  }
#else
  // Original C++ implementation
  EncoderBuffer();
  void Clear();
  void Resize(int64_t nbytes);

  bool StartBitEncoding(int64_t required_bits, bool encode_size);
  void EndBitEncoding();
  bool EncodeLeastSignificantBits32(int nbits, uint32_t value) {
    if (!bit_encoder_active()) {
      return false;
    }
    bit_encoder_->PutBits(value, nbits);
    return true;
  }
#endif
  // Encode an arbitrary data type.
  // Can be used only when we are not encoding a bit-sequence.
  // Returns false when the value couldn't be encoded.
  template <typename T>
  bool Encode(const T &data) {
#ifdef DRACO_RUST_CORE
    return draco_encoder_buffer_encode(rust_buffer_,
                                       reinterpret_cast<const uint8_t*>(&data),
                                       sizeof(T)) == DRACO_STATUS_OK;
#else
    if (bit_encoder_active()) {
      return false;
    }
    const uint8_t *src_data = reinterpret_cast<const uint8_t *>(&data);
    buffer_.insert(buffer_.end(), src_data, src_data + sizeof(T));
    return true;
#endif
  }

  bool Encode(const void *data, size_t data_size) {
#ifdef DRACO_RUST_CORE
    return draco_encoder_buffer_encode(rust_buffer_,
                                       reinterpret_cast<const uint8_t*>(data),
                                       data_size) == DRACO_STATUS_OK;
#else
    if (bit_encoder_active()) {
      return false;
    }
    const uint8_t *src_data = reinterpret_cast<const uint8_t *>(data);
    buffer_.insert(buffer_.end(), src_data, src_data + data_size);
    return true;
#endif
  }

#ifdef DRACO_RUST_CORE
  bool bit_encoder_active() const {
    return draco_encoder_buffer_bit_encoder_active(rust_buffer_);
  }
  const char *data() const {
    return reinterpret_cast<const char*>(draco_encoder_buffer_data(rust_buffer_));
  }
  size_t size() const {
    return draco_encoder_buffer_size(rust_buffer_);
  }
  std::vector<char> *buffer() {
    // For Rust implementation, we need to provide a temporary buffer
    // This is a compatibility layer for existing code
    temp_buffer_.clear();
    temp_buffer_.reserve(size());
    const char* rust_data = data();
    temp_buffer_.insert(temp_buffer_.end(), rust_data, rust_data + size());
    return &temp_buffer_;
  }
#else
  bool bit_encoder_active() const { return bit_encoder_reserved_bytes_ > 0; }
  const char *data() const { return buffer_.data(); }
  size_t size() const { return buffer_.size(); }
  std::vector<char> *buffer() { return &buffer_; }
#endif

 private:
#ifdef DRACO_RUST_CORE
  // Rust buffer handle
  draco_encoder_buffer_t* rust_buffer_;

  // Temporary buffer for compatibility with existing code
  mutable std::vector<char> temp_buffer_;
#else
  // Original C++ implementation
  // Internal helper class to encode bits to a bit buffer.
  class BitEncoder {
   public:
    // |data| is the buffer to write the bits into.
    explicit BitEncoder(char *data) : bit_buffer_(data), bit_offset_(0) {}

    // Write |nbits| of |data| into the bit buffer.
    void PutBits(uint32_t data, int32_t nbits) {
      DRACO_DCHECK_GE(nbits, 0);
      DRACO_DCHECK_LE(nbits, 32);
      for (int32_t bit = 0; bit < nbits; ++bit) {
        PutBit((data >> bit) & 1);
      }
    }

    // Return number of bits encoded so far.
    uint64_t Bits() const { return static_cast<uint64_t>(bit_offset_); }

    // TODO(fgalligan): Remove this function once we know we do not need the
    // old API anymore.
    // This is a function of an old API, that currently does nothing.
    void Flush(int /* left_over_bit_value */) {}

    // Return the number of bits required to store the given number
    static uint32_t BitsRequired(uint32_t x) {
      return static_cast<uint32_t>(MostSignificantBit(x));
    }

   private:
    void PutBit(uint8_t value) {
      const int byte_size = 8;
      const uint64_t off = static_cast<uint64_t>(bit_offset_);
      const uint64_t byte_offset = off / byte_size;
      const int bit_shift = off % byte_size;

      // TODO(fgalligan): Check performance if we add a branch and only do one
      // memory write if bit_shift is 7. Also try using a temporary variable to
      // hold the bits before writing to the buffer.

      bit_buffer_[byte_offset] &= ~(1 << bit_shift);
      bit_buffer_[byte_offset] |= value << bit_shift;
      bit_offset_++;
    }

    char *bit_buffer_;
    size_t bit_offset_;
  };
  friend class BufferBitCodingTest;
  // All data is stored in this vector.
  std::vector<char> buffer_;

  // Bit encoder is used when encoding variable-length bit data.
  // TODO(ostava): Currently encoder needs to be recreated each time
  // StartBitEncoding method is called. This is not necessary if BitEncoder
  // supported reset function which can easily added but let's leave that for
  // later.
  std::unique_ptr<BitEncoder> bit_encoder_;

  // The number of bytes reserved for bit encoder.
  // Values > 0 indicate we are in the bit encoding mode.
  int64_t bit_encoder_reserved_bytes_;

  // Flag used indicating that we need to store the length of the currently
  // processed bit sequence.
  bool encode_bit_sequence_size_;
#endif
};

}  // namespace draco

#endif  // DRACO_CORE_ENCODER_BUFFER_H_
