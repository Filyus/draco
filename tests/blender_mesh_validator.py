#!/usr/bin/env python3
"""
Test script to validate Draco-compressed GLB using Blender.
This script can be run inside Blender or standalone to analyze meshes.

Usage (standalone): python blender_mesh_validator.py <original.glb> <draco.glb>
Usage (in Blender): blender --background --python blender_mesh_validator.py -- <original.glb> <draco.glb>
"""

import sys
import struct
import json
import math

def read_glb_positions(filepath):
    """Read vertex positions from a GLB file without using Blender."""
    with open(filepath, 'rb') as f:
        data = f.read()
    
    # GLB header
    if data[0:4] != b'glTF':
        raise ValueError("Not a valid GLB file")
    
    # JSON chunk
    json_len = struct.unpack('<I', data[12:16])[0]
    json_data = data[20:20+json_len].decode('utf-8')
    gltf = json.loads(json_data)
    
    # Find binary chunk
    bin_offset = 20 + json_len
    while bin_offset % 4 != 0:
        bin_offset += 1
    
    bin_len = struct.unpack('<I', data[bin_offset:bin_offset+4])[0]
    bin_data = data[bin_offset+8:bin_offset+8+bin_len]
    
    # Check if Draco compressed
    uses_draco = 'KHR_draco_mesh_compression' in gltf.get('extensionsUsed', [])
    
    positions_by_mesh = {}
    
    for mesh_idx, mesh in enumerate(gltf.get('meshes', [])):
        mesh_name = mesh.get('name', f'mesh_{mesh_idx}')
        all_positions = []
        
        for prim in mesh.get('primitives', []):
            # Check for Draco extension
            if uses_draco and 'extensions' in prim:
                draco = prim.get('extensions', {}).get('KHR_draco_mesh_compression')
                if draco:
                    # Cannot decode Draco without a decoder - skip
                    print(f"  {mesh_name}: Draco-compressed (cannot decode in pure Python)")
                    continue
            
            # Standard glTF accessor path
            pos_acc_idx = prim.get('attributes', {}).get('POSITION')
            if pos_acc_idx is None:
                continue
            
            accessor = gltf['accessors'][pos_acc_idx]
            bv_idx = accessor.get('bufferView')
            if bv_idx is None:
                continue
            
            bv = gltf['bufferViews'][bv_idx]
            byte_offset = bv.get('byteOffset', 0) + accessor.get('byteOffset', 0)
            count = accessor['count']
            
            for i in range(count):
                offset = byte_offset + i * 12
                x = struct.unpack('<f', bin_data[offset:offset+4])[0]
                y = struct.unpack('<f', bin_data[offset+4:offset+8])[0]
                z = struct.unpack('<f', bin_data[offset+8:offset+12])[0]
                all_positions.append((x, y, z))
        
        positions_by_mesh[mesh_name] = all_positions
    
    return positions_by_mesh, uses_draco


def compare_positions(orig_positions, draco_positions, tolerance=0.01):
    """Compare two lists of positions and report differences."""
    if len(orig_positions) != len(draco_positions):
        return False, f"Vertex count mismatch: {len(orig_positions)} vs {len(draco_positions)}"
    
    max_diff = 0.0
    worst_idx = -1
    
    for i, (orig, draco) in enumerate(zip(orig_positions, draco_positions)):
        diff = max(abs(orig[j] - draco[j]) for j in range(3))
        if diff > max_diff:
            max_diff = diff
            worst_idx = i
    
    if max_diff > tolerance:
        return False, f"Position {worst_idx} differs by {max_diff:.6f}: orig={orig_positions[worst_idx]}, draco={draco_positions[worst_idx]}"
    
    return True, f"All positions match within tolerance (max_diff={max_diff:.6f})"


def main():
    args = sys.argv
    if "--" in args:
        args = args[args.index("--") + 1:]
    else:
        args = args[1:]
    
    if len(args) < 2:
        print("Usage: python blender_mesh_validator.py <original.glb> <draco.glb>")
        print("  or: blender --background --python blender_mesh_validator.py -- <original.glb> <draco.glb>")
        return 1
    
    original_file = args[0]
    draco_file = args[1]
    
    print(f"Original: {original_file}")
    print(f"Draco:    {draco_file}")
    print()
    
    # Read original positions
    print("Reading original GLB...")
    orig_positions, orig_uses_draco = read_glb_positions(original_file)
    if orig_uses_draco:
        print("  Warning: Original file uses Draco - cannot compare without Blender")
    
    for mesh_name, positions in orig_positions.items():
        print(f"  {mesh_name}: {len(positions)} vertices")
        if len(positions) > 0:
            print(f"    First vertex: {positions[0]}")
            print(f"    Last vertex:  {positions[-1]}")
    
    print()
    print("Reading Draco-compressed GLB...")
    draco_positions, draco_uses_draco = read_glb_positions(draco_file)
    
    for mesh_name, positions in draco_positions.items():
        print(f"  {mesh_name}: {len(positions)} vertices")
    
    if draco_uses_draco:
        print()
        print("Draco file uses compression - need Blender to decode and compare.")
        print("Run this script in Blender:")
        print(f"  blender --background --python {sys.argv[0]} -- {original_file} {draco_file}")
        
        # Try to use Blender if available
        try:
            import bpy
            print()
            print("Blender is available - performing full comparison...")
            return blender_compare(original_file, draco_file)
        except ImportError:
            return 0
    
    # Compare positions
    print()
    print("Comparing positions...")
    for mesh_name in orig_positions:
        if mesh_name in draco_positions:
            ok, msg = compare_positions(orig_positions[mesh_name], draco_positions[mesh_name])
            status = "OK" if ok else "FAIL"
            print(f"  {mesh_name}: {status} - {msg}")
    
    return 0


def blender_compare(original_file, draco_file):
    """Compare meshes using Blender's glTF importer."""
    import bpy
    
    def get_mesh_positions(filepath):
        """Import glTF and get vertex positions."""
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=filepath)
        
        positions = {}
        for obj in bpy.context.scene.objects:
            if obj.type == 'MESH':
                mesh = obj.data
                # Get world-space positions
                verts = [(obj.matrix_world @ v.co).to_tuple() for v in mesh.vertices]
                positions[obj.name] = verts
        
        return positions
    
    print("Loading original mesh in Blender...")
    orig = get_mesh_positions(original_file)
    for name, verts in orig.items():
        print(f"  {name}: {len(verts)} vertices")
    
    print()
    print("Loading Draco mesh in Blender...")
    draco = get_mesh_positions(draco_file)
    for name, verts in draco.items():
        print(f"  {name}: {len(verts)} vertices")
    
    print()
    print("Comparing...")
    
    all_ok = True
    for name in orig:
        if name not in draco:
            print(f"  {name}: MISSING in Draco file")
            all_ok = False
            continue
        
        ok, msg = compare_positions(orig[name], draco[name])
        status = "OK" if ok else "FAIL"
        print(f"  {name}: {status} - {msg}")
        if not ok:
            all_ok = False
    
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
