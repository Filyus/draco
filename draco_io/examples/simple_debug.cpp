// Simple debug test for file reading
#include "draco/core/status.h"
#include "draco/core/status_or.h"
#include "draco/mesh/mesh.h"
#include "draco/io/mesh_io.h"
#include "draco/io/point_cloud_io.h"
#include <iostream>
#include <string>
#include <memory>
using namespace draco;

int main() {
    std::cout << "Simple Debug Test\n";
    std::cout << "=================\n\n";

    std::string test_file = "C:/Projects/Draco/testdata/Box.ply";

    std::cout << "Testing file: " << test_file << "\n";

    // Get the extension manually to test LowercaseFileExtension
    size_t pos = test_file.find_last_of('.');
    std::string extension;
    if (pos != std::string::npos && pos > 0 && pos < test_file.length() - 1) {
        extension = test_file.substr(pos + 1);
        // Convert to lowercase
        for (char& c : extension) {
            c = tolower(c);
        }
    }

    std::cout << "Detected extension: '" << extension << "'\n";

    // Test with Draco's ReadMeshFromFile
    std::cout << "\nCalling ReadMeshFromFile...\n";
    auto result = ReadMeshFromFile(test_file);

    if (result.ok()) {
        auto mesh = std::move(result).value();
        std::cout << "✅ SUCCESS!\n";
        std::cout << "Points: " << mesh->num_points() << "\n";
        std::cout << "Faces: " << mesh->num_faces() << "\n";
    } else {
        std::cout << "❌ FAILED!\n";
        std::cout << "Error: " << result.status().error_msg() << "\n";
        std::cout << "Error code: " << result.status().code() << "\n";
    }

    return 0;
}