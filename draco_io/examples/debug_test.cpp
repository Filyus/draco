// Debug test to understand file reading issues
#include "draco/core/status.h"
#include "draco/core/status_or.h"
#include "draco/io/mesh_io.h"
#include "draco/io/point_cloud_io.h"
#include <iostream>
#include <fstream>

int main() {
    std::cout << "Debug Test: Understanding file reading issues\n";
    std::cout << "=============================================\n\n";

    // Test 1: Check if file exists and is readable directly
    std::string test_file = "C:/Projects/Draco/testdata/Box.ply";
    std::ifstream direct_file(test_file, std::ios::binary);

    std::cout << "1. Direct file access test:\n";
    if (direct_file.good()) {
        direct_file.seekg(0, std::ios::end);
        size_t size = direct_file.tellg();
        direct_file.close();
        std::cout << "   ✅ Direct access: File exists, " << size << " bytes\n";
    } else {
        std::cout << "   ❌ Direct access: Cannot read file\n";
        return 1;
    }

    // Test 2: Try using Draco's ReadMeshFromFile
    std::cout << "\n2. Draco ReadMeshFromFile test:\n";
    auto mesh_result = draco::ReadMeshFromFile(test_file);

    if (mesh_result.ok()) {
        auto mesh = std::move(mesh_result).value();
        std::cout << "   ✅ Draco read: Success!\n";
        std::cout << "   📊 Points: " << mesh->num_points() << "\n";
        std::cout << "   📊 Faces: " << mesh->num_faces() << "\n";
    } else {
        std::cout << "   ❌ Draco read: Failed\n";
        std::cout << "   💥 Error: " << mesh_result.status().error_msg() << "\n";
    }

    // Test 3: Try with a simple OBJ file
    std::cout << "\n3. Test with OBJ file:\n";
    std::string obj_file = "C:/Projects/Draco/testdata/cube_att.obj";
    auto obj_result = draco::ReadMeshFromFile(obj_file);

    if (obj_result.ok()) {
        auto mesh = std::move(obj_result).value();
        std::cout << "   ✅ OBJ read: Success!\n";
        std::cout << "   📊 Points: " << mesh->num_points() << "\n";
        std::cout << "   📊 Faces: " << mesh->num_faces() << "\n";
    } else {
        std::cout << "   ❌ OBJ read: Failed\n";
        std::cout << "   💥 Error: " << obj_result.status().error_msg() << "\n";
    }

    return 0;
}