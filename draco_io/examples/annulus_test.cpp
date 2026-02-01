// Test program to encode/decode annulus.obj with original Google Draco
// Compare with draco_core implementation

#include "draco/compression/decode.h"
#include "draco/compression/encode.h"
#include "draco/core/decoder_buffer.h"
#include "draco/core/encoder_buffer.h"
#include "draco/io/mesh_io.h"
#include "draco/io/obj_decoder.h"
#include "draco/io/obj_encoder.h"
#include "draco/mesh/mesh.h"

#include <fstream>
#include <iostream>
#include <memory>
#include <vector>

using namespace draco;

int main(int argc, char** argv) {
  if (argc != 4) {
    std::cout << "Usage: " << argv[0]
              << " <input.obj> <output.drc> <decoded.obj>\n";
    std::cout << "Example: " << argv[0]
              << " annulus.obj test.drc decoded.obj\n";
    return 1;
  }

  const std::string input_obj = argv[1];
  const std::string output_drc = argv[2];
  const std::string decoded_obj = argv[3];

  std::cout << "\n=== Original Google Draco Test ===\n\n";

  // Step 1: Load OBJ file
  std::cout << "1. Loading OBJ file: " << input_obj << "\n";
  ObjDecoder obj_decoder;
  std::unique_ptr<Mesh> input_mesh(new Mesh());
  
  auto status = obj_decoder.DecodeFromFile(input_obj, input_mesh.get());
  if (!status.ok()) {
    std::cerr << "   ERROR: Failed to load OBJ: " << status.error_msg() << "\n";
    return 1;
  }

  std::cout << "   Input mesh statistics:\n";
  std::cout << "   - Points: " << input_mesh->num_points() << "\n";
  std::cout << "   - Faces: " << input_mesh->num_faces() << "\n";
  std::cout << "   - Attributes: " << input_mesh->num_attributes() << "\n";

  // Print vertex positions
  const auto* pos_attr = input_mesh->GetNamedAttribute(GeometryAttribute::POSITION);
  if (pos_attr) {
    std::cout << "   - Position attribute values: " << pos_attr->size() << "\n";
    std::cout << "   Input vertices (first 10):\n";
    for (int i = 0; i < std::min(10, (int)input_mesh->num_points()); ++i) {
      std::array<float, 3> pos;
      pos_attr->GetValue(AttributeValueIndex(i), pos.data());
      std::cout << "     v" << i << ": (" << pos[0] << ", " << pos[1] << ", "
                << pos[2] << ")\n";
    }
  }

  // Step 2: Encode to Draco
  std::cout << "\n2. Encoding to Draco format...\n";
  Encoder encoder;
  encoder.SetSpeedOptions(5, 5);
  encoder.SetAttributeQuantization(GeometryAttribute::POSITION, 14);

  EncoderBuffer buffer;
  status = encoder.EncodeMeshToBuffer(*input_mesh, &buffer);
  if (!status.ok()) {
    std::cerr << "   ERROR: Encoding failed: " << status.error_msg() << "\n";
    return 1;
  }

  std::cout << "   Encoded size: " << buffer.size() << " bytes\n";

  // Save encoded data
  std::ofstream out_file(output_drc, std::ios::binary);
  if (!out_file) {
    std::cerr << "   ERROR: Cannot open output file: " << output_drc << "\n";
    return 1;
  }
  out_file.write(buffer.data(), buffer.size());
  out_file.close();
  std::cout << "   Saved to: " << output_drc << "\n";

  // Step 3: Decode from Draco
  std::cout << "\n3. Decoding from Draco format...\n";
  DecoderBuffer dec_buffer;
  dec_buffer.Init(buffer.data(), buffer.size());

  Decoder decoder;
  auto decode_result = decoder.DecodeMeshFromBuffer(&dec_buffer);
  if (!decode_result.ok()) {
    std::cerr << "   ERROR: Decoding failed: " << decode_result.status().error_msg()
              << "\n";
    return 1;
  }

  std::unique_ptr<Mesh> decoded_mesh = std::move(decode_result).value();
  std::cout << "   Decoded mesh statistics:\n";
  std::cout << "   - Points: " << decoded_mesh->num_points() << "\n";
  std::cout << "   - Faces: " << decoded_mesh->num_faces() << "\n";
  std::cout << "   - Attributes: " << decoded_mesh->num_attributes() << "\n";

  // Print decoded vertices
  const auto* dec_pos_attr =
      decoded_mesh->GetNamedAttribute(GeometryAttribute::POSITION);
  if (dec_pos_attr) {
    std::cout << "   - Position attribute values: " << dec_pos_attr->size()
              << "\n";
    std::cout << "   Decoded vertices (first 10):\n";
    for (int i = 0; i < std::min(10, (int)decoded_mesh->num_points()); ++i) {
      std::array<float, 3> pos;
      dec_pos_attr->GetValue(AttributeValueIndex(i), pos.data());
      std::cout << "     v" << i << ": (" << pos[0] << ", " << pos[1] << ", "
                << pos[2] << ")\n";
    }
  }

  // Step 4: Save decoded mesh as OBJ
  std::cout << "\n4. Saving decoded mesh to: " << decoded_obj << "\n";
  ObjEncoder obj_encoder;
  bool success = obj_encoder.EncodeToFile(*decoded_mesh, decoded_obj);
  if (!success) {
    std::cerr << "   ERROR: Failed to save OBJ\n";
    return 1;
  }

  // Step 5: Analysis
  std::cout << "\n=== Analysis ===\n";
  std::cout << "Input points:   " << input_mesh->num_points() << "\n";
  std::cout << "Decoded points: " << decoded_mesh->num_points() << "\n";

  if (input_mesh->num_points() == decoded_mesh->num_points()) {
    std::cout << "✅ PASS: Point count matches (no duplicate vertices)\n";
  } else {
    std::cout << "❌ FAIL: Point count mismatch!\n";
    std::cout << "   Expected: " << input_mesh->num_points() << "\n";
    std::cout << "   Got:      " << decoded_mesh->num_points() << "\n";
    std::cout << "   Difference: +"
              << (decoded_mesh->num_points() - input_mesh->num_points())
              << " duplicate vertices\n";
  }

  std::cout << "\n✅ Test complete!\n";
  std::cout << "Compare with draco_core output to verify behavior.\n\n";

  return 0;
}
