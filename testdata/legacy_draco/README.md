# Legacy Draco compatibility fixtures

These fixtures are small smoke-test assets generated with local C++ Draco tools
from `draco-version-tools/`. They intentionally cover the Rust decoder policy
floor of Draco 1.0.0+ without adding a full version matrix.

| Fixture | Source | Encoder | Command options | Expected header |
| --- | --- | --- | --- | --- |
| `cube_att.mesh_seq.1.0.0.drc` | `../cube_att.obj` | `1.0.0-b756664/draco_encoder.exe` | `-cl 0` | `v2.0 mesh method=0` |
| `cube_att.mesh_eb.1.0.0.drc` | `../cube_att.obj` | `1.0.0-b756664/draco_encoder.exe` | `-cl 10` | `v2.0 mesh method=1` |
| `cube_att.mesh_seq.1.1.0.drc` | `../cube_att.obj` | `1.1.0-dc28e6a/draco_encoder.exe` | `-cl 0` | `v2.1 mesh method=0` |
| `cube_att.mesh_eb.1.1.0.drc` | `../cube_att.obj` | `1.1.0-dc28e6a/draco_encoder.exe` | `-cl 10` | `v2.1 mesh method=1` |
| `point_cloud_pos_norm.seq.1.0.0.drc` | `../point_cloud_test_pos_norm.ply` | `1.0.0-b756664/draco_encoder.exe` | `-point_cloud -cl 0` | `v2.0 point_cloud method=0` |
| `point_cloud_pos_norm.seq.1.1.0.drc` | `../point_cloud_test_pos_norm.ply` | `1.1.0-dc28e6a/draco_encoder.exe` | `-point_cloud -cl 0` | `v2.1 point_cloud method=0` |
