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
#ifndef DRACO_CORE_DECODER_BUFFER_H_
#define DRACO_CORE_DECODER_BUFFER_H_

#include <stdint.h>

#include <cstring>
#include <memory>

#include "draco/core/macros.h"
#include "draco/draco_features.h"

#ifdef DRACO_RUST_CORE
// Include Rust C API header when Rust support is enabled
extern "C" {
#include "draco_core.h"
}
#endif

namespace draco {

// Class is a wrapper around input data used by MeshDecoder. It provides a
// basic interface for decoding either typed or variable-bit sized data.
class DecoderBuffer {
 public:
#ifdef DRACO_RUST_CORE
  // Constructor that uses Rust implementation
  DecoderBuffer() : rust_buffer_(draco_decoder_buffer_create()) {}

  // Destructor
  ~DecoderBuffer() {
    if (rust_buffer_) {
      draco_decoder_buffer_destroy(rust_buffer_);
    }
  }

  // Copy constructor
  DecoderBuffer(const DecoderBuffer &other) : rust_buffer_(draco_decoder_buffer_create()) {
    // Note: We can't easily copy the data from Rust buffer without knowing the original data
    // This is a limitation of the Rust integration design
  }

  // Assignment operator
  DecoderBuffer &operator=(const DecoderBuffer &other) {
    if (this != &other) {
      // Recreate buffer
      if (rust_buffer_) {
        draco_decoder_buffer_destroy(rust_buffer_);
      }
      rust_buffer_ = draco_decoder_buffer_create();
      // Note: Can't easily copy data without original data reference
    }
    return *this;
  }

  void Init(const char *data, size_t data_size) {
    draco_decoder_buffer_init(rust_buffer_, reinterpret_cast<const uint8_t*>(data), data_size);
  }

  void Init(const char *data, size_t data_size, uint16_t version) {
    draco_decoder_buffer_init_with_version(rust_buffer_, reinterpret_cast<const uint8_t*>(data), data_size, version);
  }

  bool StartBitDecoding(bool decode_size, uint64_t *out_size) {
    return draco_decoder_buffer_start_bit_decoding(rust_buffer_, decode_size, out_size) == DRACO_STATUS_OK;
  }

  void EndBitDecoding() {
    draco_decoder_buffer_end_bit_decoding(rust_buffer_);
  }

  bool DecodeLeastSignificantBits32(uint32_t nbits, uint32_t *out_value) {
    return draco_decoder_buffer_decode_bits(rust_buffer_, nbits, out_value) == DRACO_STATUS_OK;
  }
#else
  // Original C++ implementation
  DecoderBuffer();
  DecoderBuffer(const DecoderBuffer &buf) = default;

  DecoderBuffer &operator=(const DecoderBuffer &buf) = default;

  void Init(const char *data, size_t data_size);

  void Init(const char *data, size_t data_size, uint16_t version);

  bool StartBitDecoding(bool decode_size, uint64_t *out_size);

  void EndBitDecoding();

  bool DecodeLeastSignificantBits32(uint32_t nbits, uint32_t *out_value) {
    if (!bit_decoder_active()) {
      return false;
    }
    return bit_decoder_.GetBits(nbits, out_value);
  }
#endif

  // Decodes an arbitrary data type.
  // Can be used only when we are not decoding a bit-sequence.
  // Returns false on error.
  template <typename T>
  bool Decode(T *out_val) {
#ifdef DRACO_RUST_CORE
    // For now, use C++ fallback for complex template operations
    return Decode(out_val, sizeof(T));
#else
    if (!Peek(out_val)) {
      return false;
    }
    pos_ += sizeof(T);
    return true;
#endif
  }

  bool Decode(void *out_data, size_t size_to_decode) {
#ifdef DRACO_RUST_CORE
    return draco_decoder_buffer_decode(rust_buffer_, static_cast<uint8_t*>(out_data), size_to_decode) == DRACO_STATUS_OK;
#else
    if (data_size_ < static_cast<int64_t>(pos_ + size_to_decode)) {
      return false;  // Buffer overflow.
    }
    memcpy(out_data, (data_ + pos_), size_to_decode);
    pos_ += size_to_decode;
    return true;
#endif
  }

  // Decodes an arbitrary data, but does not advance the reading position.
  template <typename T>
  bool Peek(T *out_val) {
#ifdef DRACO_RUST_CORE
    // For now, use C++ fallback for complex template operations
    return Peek(out_val, sizeof(T));
#else
    const size_t size_to_decode = sizeof(T);
    if (data_size_ < static_cast<int64_t>(pos_ + size_to_decode)) {
      return false;  // Buffer overflow.
    }
    memcpy(out_val, (data_ + pos_), size_to_decode);
    return true;
#endif
  }

  bool Peek(void *out_data, size_t size_to_peek) {
#ifdef DRACO_RUST_CORE
    return draco_decoder_buffer_peek(rust_buffer_, static_cast<uint8_t*>(out_data), size_to_peek) == DRACO_STATUS_OK;
#else
    if (data_size_ < static_cast<int64_t>(pos_ + size_to_peek)) {
      return false;  // Buffer overflow.
    }
    memcpy(out_data, (data_ + pos_), size_to_peek);
    return true;
#endif
  }

  // Discards #bytes from the input buffer.
  void Advance(int64_t bytes) {
#ifdef DRACO_RUST_CORE
    // Note: Need to implement advance in Rust API
    // For now, use C++ fallback or track position separately
    if (bytes >= 0) {
      pos_ += bytes;
    }
#else
    pos_ += bytes;
#endif
  }

  // Moves the parsing position to a specific offset from the beginning of the
  // input data.
  void StartDecodingFrom(int64_t offset) {
#ifdef DRACO_RUST_CORE
    pos_ = offset;  // Track position for compatibility
#else
    pos_ = offset;
#endif
  }

  void set_bitstream_version(uint16_t version) {
#ifdef DRACO_RUST_CORE
    bitstream_version_ = version;  // Store for compatibility
#else
    bitstream_version_ = version;
#endif
  }

  // Returns the data array at the current decoder position.
  const char *data_head() const {
#ifdef DRACO_RUST_CORE
    // For Rust implementation, we need to calculate head position
    return reinterpret_cast<const char*>(data_head_address);
#else
    return data_ + pos_;
#endif
  }

  int64_t remaining_size() const {
#ifdef DRACO_RUST_CORE
    return static_cast<int64_t>(draco_decoder_buffer_remaining_size(rust_buffer_));
#else
    return data_size_ - pos_;
#endif
  }

  int64_t decoded_size() const {
#ifdef DRACO_RUST_CORE
    return static_cast<int64_t>(draco_decoder_buffer_position(rust_buffer_));
#else
    return pos_;
#endif
  }

  bool bit_decoder_active() const {
#ifdef DRACO_RUST_CORE
    return draco_decoder_buffer_bit_decoder_active(rust_buffer_);
#else
    return bit_mode_;
#endif
  }

  // Returns the bitstream associated with the data. Returns 0 if unknown.
  uint16_t bitstream_version() const {
#ifdef DRACO_RUST_CORE
    return bitstream_version_;
#else
    return bitstream_version_;
#endif
  }

 private:
#ifdef DRACO_RUST_CORE
  // Rust buffer handle
  draco_decoder_buffer_t* rust_buffer_;

  // Compatibility data for C++ fallback operations
  const char* data_head_address;
  int64_t pos_;
  uint16_t bitstream_version_;
#else
  // Internal helper class to decode bits from a bit buffer.
  class BitDecoder {
   public:
    BitDecoder();
    ~BitDecoder();

    // Sets the bit buffer to |b|. |s| is the size of |b| in bytes.
    inline void reset(const void *b, size_t s) {
      bit_offset_ = 0;
      bit_buffer_ = static_cast<const uint8_t *>(b);
      bit_buffer_end_ = bit_buffer_ + s;
    }

    // Returns number of bits decoded so far.
    inline uint64_t BitsDecoded() const {
      return static_cast<uint64_t>(bit_offset_);
    }

    // Return number of bits available for decoding
    inline uint64_t AvailBits() const {
      return ((bit_buffer_end_ - bit_buffer_) * 8) - bit_offset_;
    }

    inline uint32_t EnsureBits(int k) {
      DRACO_DCHECK_LE(k, 24);
      DRACO_DCHECK_LE(static_cast<uint64_t>(k), AvailBits());

      uint32_t buf = 0;
      for (int i = 0; i < k; ++i) {
        buf |= PeekBit(i) << i;
      }
      return buf;  // Okay to return extra bits
    }

    inline void ConsumeBits(int k) { bit_offset_ += k; }

    // Returns |nbits| bits in |x|.
    inline bool GetBits(uint32_t nbits, uint32_t *x) {
      if (nbits > 32) {
        return false;
      }
      uint32_t value = 0;
      for (uint32_t bit = 0; bit < nbits; ++bit) {
        value |= GetBit() << bit;
      }
      *x = value;
      return true;
    }

   private:
    // TODO(fgalligan): Add support for error reporting on range check.
    // Returns one bit from the bit buffer.
    inline int GetBit() {
      const size_t off = bit_offset_;
      const size_t byte_offset = off >> 3;
      const int bit_shift = static_cast<int>(off & 0x7);
      if (bit_buffer_ + byte_offset < bit_buffer_end_) {
        const int bit = (bit_buffer_[byte_offset] >> bit_shift) & 1;
        bit_offset_ = off + 1;
        return bit;
      }
      return 0;
    }

    inline int PeekBit(int offset) {
      const size_t off = bit_offset_ + offset;
      const size_t byte_offset = off >> 3;
      const int bit_shift = static_cast<int>(off & 0x7);
      if (bit_buffer_ + byte_offset < bit_buffer_end_) {
        const int bit = (bit_buffer_[byte_offset] >> bit_shift) & 1;
        return bit;
      }
      return 0;
    }

    const uint8_t *bit_buffer_;
    const uint8_t *bit_buffer_end_;
    size_t bit_offset_;
  };
  friend class BufferBitCodingTest;

  const char *data_;
  int64_t data_size_;

  // Current parsing position of the decoder.
  int64_t pos_;
  BitDecoder bit_decoder_;
  bool bit_mode_;
  uint16_t bitstream_version_;
#endif
};

}  // namespace draco

#endif  // DRACO_CORE_DECODER_BUFFER_H_
