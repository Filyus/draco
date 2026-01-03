// Quick test to see what happens when we try to use GLTF decoder without transcoder support

#include "draco/io/gltf_decoder.h"
#include <iostream>

int main() {
    std::cout << "Testing GLTF decoder availability...\n";

    draco::GltfDecoder decoder;
    std::cout << "GLTF decoder created successfully\n";

    auto result = decoder.DecodeFromFile("sphere.gltf");
    if (!result.ok()) {
        std::cout << "GLTF decode failed: " << result.status().error_msg() << "\n";
    } else {
        std::cout << "GLTF decode succeeded!\n";
    }

    return 0;
}