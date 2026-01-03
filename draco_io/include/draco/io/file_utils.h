// Copyright 2018 The Draco Authors.
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
#ifndef DRACO_IO_FILE_UTILS_H_
#define DRACO_IO_FILE_UTILS_H_

#include <cstdint>
#include <string>
#include <vector>

#include "draco/core/path_utils.h"

namespace draco {

// Convenience methods. Uses draco::FileReaderFactory internally. Reads contents
// of file referenced by |file_name| into |buffer| and returns true upon
// success.
bool ReadFileToBuffer(const std::string &file_name, std::vector<char> *buffer);
bool ReadFileToBuffer(const std::string &file_name,
                      std::vector<uint8_t> *buffer);

// Convenience method for reading a file into a std::string. Reads contents
// of file referenced by |file_name| into |contents| and returns true upon
// success.
bool ReadFileToString(const std::string &file_name, std::string *contents);

// Convenience method. Uses draco::FileWriterFactory internally. Writes contents
// of |buffer| to file referred to by |file_name|. File is overwritten if it
// exists. Returns true after successful write.
bool WriteBufferToFile(const char *buffer, size_t buffer_size,
                       const std::string &file_name);
bool WriteBufferToFile(const unsigned char *buffer, size_t buffer_size,
                       const std::string &file_name);
bool WriteBufferToFile(const void *buffer, size_t buffer_size,
                       const std::string &file_name);

// Convenience method. Uses draco::FileReaderFactory internally. Returns size of
// file referenced by |file_name|. Returns 0 when referenced file is empty or
// does not exist.
size_t GetFileSize(const std::string &file_name);

}  // namespace draco

#endif  // DRACO_IO_FILE_UTILS_H_
