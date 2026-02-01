#include "draco/io/file_utils.h"

#include <string>
#include <vector>

#include "draco/io/file_reader_factory.h"
#include "draco/io/file_reader_interface.h"
#include "draco/io/file_writer_factory.h"
#include "draco/io/file_writer_interface.h"
#include "draco/io/stdio_file_reader.h"
#include "draco/io/stdio_file_writer.h"

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

// Note: GetFullPath is implemented in draco_core/path_utils.cc

void InitFileIO() {
  // Force registration of standard file reader and writer by referencing
  // symbols from their translation units. The static registration pattern
  // used by StdioFileReader and StdioFileWriter may not work reliably when
  // linking as a static library, because the linker may not include object
  // files that have no referenced symbols.
  static bool initialized = false;
  if (!initialized) {
    // These calls force the linker to include the object files containing
    // StdioFileReader and StdioFileWriter, which triggers their static
    // registration with the factory classes.
    FileReaderFactory::RegisterReader(StdioFileReader::Open);
    FileWriterFactory::RegisterWriter(StdioFileWriter::Open);
    initialized = true;
  }
}

}  // namespace draco
