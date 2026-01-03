#include "draco/core/path_utils.h"
#include <algorithm>
#include <cctype>

namespace draco {
namespace {
void SplitPathPrivate(const std::string &full_path,
                      std::string *out_folder_path,
                      std::string *out_file_name) {
  const auto pos = full_path.find_last_of("/\\");
  if (pos != std::string::npos) {
    if (out_folder_path) {
      *out_folder_path = full_path.substr(0, pos + 1);
    }
    if (out_file_name) {
      *out_file_name = full_path.substr(pos + 1);
    }
  } else {
    if (out_folder_path) {
      *out_folder_path = "";
    }
    if (out_file_name) {
      *out_file_name = full_path;
    }
  }
}

std::string ToLower(const std::string &s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(),
                 [](unsigned char c) { return std::tolower(c); });
  return out;
}
} // namespace

void SplitPath(const std::string &full_path, std::string *out_folder_path,
               std::string *out_file_name) {
  SplitPathPrivate(full_path, out_folder_path, out_file_name);
}

std::string ReplaceFileExtension(const std::string &in_file_name,
                                 const std::string &new_extension) {
  const auto pos = in_file_name.find_last_of('.');
  if (pos == std::string::npos) {
    // No extension found.
    return in_file_name + "." + new_extension;
  }
  return in_file_name.substr(0, pos + 1) + new_extension;
}

std::string LowercaseFileExtension(const std::string &filename) {
  const size_t pos = filename.find_last_of('.');
  if (pos == 0 || pos == std::string::npos || pos == filename.length() - 1) {
    return "";
  }
  return ToLower(filename.substr(pos + 1));
}

std::string LowercaseMimeTypeExtension(const std::string &mime_type) {
  const size_t pos = mime_type.find_last_of('/');
  if (pos == 0 || pos == std::string::npos || pos == mime_type.length() - 1) {
    return "";
  }
  return ToLower(mime_type.substr(pos + 1));
}

std::string RemoveFileExtension(const std::string &filename) {
  const size_t pos = filename.find_last_of('.');
  if (pos == 0 || pos == std::string::npos || pos == filename.length() - 1) {
    return filename;
  }
  return filename.substr(0, pos);
}

std::string GetFullPath(const std::string &input_file_relative_path,
                        const std::string &sibling_file_full_path) {
  const auto pos = sibling_file_full_path.find_last_of("/\\");
  std::string input_file_full_path;
  if (pos != std::string::npos) {
    input_file_full_path = sibling_file_full_path.substr(0, pos + 1);
  }
  input_file_full_path += input_file_relative_path;
  return input_file_full_path;
}

}  // namespace draco
