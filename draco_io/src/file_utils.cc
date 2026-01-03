#include "draco/io/file_utils.h"

#include <string>
#include <vector>

#include "draco/io/file_reader_factory.h"
#include "draco/io/file_reader_interface.h"
#include "draco/io/file_writer_factory.h"
#include "draco/io/file_writer_interface.h"

namespace draco {

bool ReadFileToBuffer(const std::string &file_name, std::vector<char> *buffer) {
  std::unique_ptr<FileReaderInterface> file_reader =
      FileReaderFactory::OpenReader(file_name);
  if (file_reader == nullptr) {
    return false;
  }
  return file_reader->ReadFileToBuffer(buffer);
}

bool ReadFileToBuffer(const std::string &file_name,
                      std::vector<uint8_t> *buffer) {
  std::unique_ptr<FileReaderInterface> file_reader =
      FileReaderFactory::OpenReader(file_name);
  if (file_reader == nullptr) {
    return false;
  }
  return file_reader->ReadFileToBuffer(buffer);
}

bool ReadFileToString(const std::string &file_name, std::string *contents) {
  if (!contents) {
    return false;
  }
  std::unique_ptr<FileReaderInterface> file_reader =
      FileReaderFactory::OpenReader(file_name);
  if (file_reader == nullptr) {
    return false;
  }
  std::vector<char> buffer;
  if (!ReadFileToBuffer(file_name, &buffer)) {
    return false;
  }
  contents->assign(buffer.begin(), buffer.end());
  return true;
}

bool WriteBufferToFile(const char *buffer, size_t buffer_size,
                       const std::string &file_name) {
  std::unique_ptr<FileWriterInterface> file_writer =
      FileWriterFactory::OpenWriter(file_name);
  if (file_writer == nullptr) {
    return false;
  }
  return file_writer->Write(buffer, buffer_size);
}

bool WriteBufferToFile(const unsigned char *buffer, size_t buffer_size,
                       const std::string &file_name) {
  return WriteBufferToFile(reinterpret_cast<const char *>(buffer), buffer_size,
                           file_name);
}

bool WriteBufferToFile(const void *buffer, size_t buffer_size,
                       const std::string &file_name) {
  return WriteBufferToFile(reinterpret_cast<const char *>(buffer), buffer_size,
                           file_name);
}

size_t GetFileSize(const std::string &file_name) {
  std::unique_ptr<FileReaderInterface> file_reader =
      FileReaderFactory::OpenReader(file_name);
  if (file_reader == nullptr) {
    return 0;
  }
  return file_reader->GetFileSize();
}

}  // namespace draco
