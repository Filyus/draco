#!/usr/bin/env python3
"""
Blender test script to validate Draco-compressed GLB files.

Usage: blender --background --python blender_draco_test.py -- <input.glb> <output.glb>

This script compares vertex positions between original and Draco-compressed meshes.
"""

import sys
import os

# Get arguments after --
argv = sys.argv
if "--" in argv:
    argv = argv[argv.index("--") + 1:]
else:
    argv = []

def main():
    try:
        import bpy
    except ImportError:
        print("ERROR: This script must be run from Blender")
        print("Usage: blender --background --python blender_draco_test.py -- <input.glb> <output.glb>")
        sys.exit(1)
    
    if len(argv) < 2:
        print("Usage: blender --background --python blender_draco_test.py -- <input.glb> <output.glb>")
        sys.exit(1)
    
    input_file = argv[0]
    output_file = argv[1]
    
    print(f"\n=== Draco GLB Validation Test ===")
    print(f"Input:  {input_file}")
    print(f"Output: {output_file}")
    
    # Clear scene
    bpy.ops.wm.read_factory_settings(use_empty=True)
    
    # Import original GLB
    print(f"\n--- Loading original file ---")
    bpy.ops.import_scene.gltf(filepath=input_file)
    
    original_meshes = {}
    for obj in bpy.context.scene.objects:
        if obj.type == 'MESH':
            mesh = obj.data
            # Get world-space vertices
            verts = [(obj.matrix_world @ v.co).to_tuple() for v in mesh.vertices]
            faces = [tuple(p.vertices) for p in mesh.polygons]
            original_meshes[obj.name] = {
                'vertices': verts,
                'faces': faces,
                'vertex_count': len(mesh.vertices),
                'face_count': len(mesh.polygons),
            }
            print(f"  {obj.name}: {len(mesh.vertices)} vertices, {len(mesh.polygons)} faces")
    
    # Clear scene again
    bpy.ops.wm.read_factory_settings(use_empty=True)
    
    # Import output GLB
    print(f"\n--- Loading output file ---")
    try:
        bpy.ops.import_scene.gltf(filepath=output_file)
    except Exception as e:
        print(f"ERROR: Failed to import output file: {e}")
        sys.exit(1)
    
    output_meshes = {}
    for obj in bpy.context.scene.objects:
        if obj.type == 'MESH':
            mesh = obj.data
            verts = [(obj.matrix_world @ v.co).to_tuple() for v in mesh.vertices]
            faces = [tuple(p.vertices) for p in mesh.polygons]
            output_meshes[obj.name] = {
                'vertices': verts,
                'faces': faces,
                'vertex_count': len(mesh.vertices),
                'face_count': len(mesh.polygons),
            }
            print(f"  {obj.name}: {len(mesh.vertices)} vertices, {len(mesh.polygons)} faces")
    
    # Compare meshes
    print(f"\n--- Comparison ---")
    errors = []
    
    for name, orig in original_meshes.items():
        if name not in output_meshes:
            errors.append(f"Mesh '{name}' missing in output")
            continue
        
        out = output_meshes[name]
        
        # Check vertex count
        if orig['vertex_count'] != out['vertex_count']:
            errors.append(f"Mesh '{name}': vertex count mismatch ({orig['vertex_count']} vs {out['vertex_count']})")
        
        # Check face count
        if orig['face_count'] != out['face_count']:
            errors.append(f"Mesh '{name}': face count mismatch ({orig['face_count']} vs {out['face_count']})")
        
        # Check vertex positions (with tolerance for quantization)
        if orig['vertex_count'] == out['vertex_count']:
            max_diff = 0.0
            worst_vertex = -1
            for i, (v1, v2) in enumerate(zip(orig['vertices'], out['vertices'])):
                diff = max(abs(v1[j] - v2[j]) for j in range(3))
                if diff > max_diff:
                    max_diff = diff
                    worst_vertex = i
            
            # Expected error for 14-bit quantization depends on mesh size
            # Calculate bounding box
            all_coords = [c for v in orig['vertices'] for c in v]
            if all_coords:
                bbox_size = max(all_coords) - min(all_coords)
                expected_error = bbox_size / (2**14)  # 14-bit quantization
                
                print(f"  {name}: bbox={bbox_size:.4f}, expected_error={expected_error:.6f}, max_diff={max_diff:.6f}")
                
                if max_diff > expected_error * 10:  # Allow 10x tolerance
                    errors.append(f"Mesh '{name}': vertex positions differ too much (max_diff={max_diff:.6f}, expected<{expected_error:.6f})")
                    print(f"    Worst vertex index: {worst_vertex}")
                    if worst_vertex >= 0 and worst_vertex < len(orig['vertices']):
                        print(f"    Original: {orig['vertices'][worst_vertex]}")
                        print(f"    Output:   {out['vertices'][worst_vertex]}")
    
    # Summary
    print(f"\n--- Summary ---")
    if errors:
        print(f"FAILED: {len(errors)} errors")
        for e in errors:
            print(f"  - {e}")
        sys.exit(1)
    else:
        print("PASSED: All meshes match within tolerance")
        sys.exit(0)

if __name__ == "__main__":
    main()
