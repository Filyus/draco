use crate::geometry_indices::{FaceIndex, PointIndex};
use crate::point_cloud::PointCloud;
use std::ops::{Deref, DerefMut};

pub type Face = [PointIndex; 3];

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    point_cloud: PointCloud,
    faces: Vec<Face>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_face(&mut self, face: Face) {
        self.faces.push(face);
    }

    pub fn set_face(&mut self, face_id: FaceIndex, face: Face) {
        if face_id.0 as usize >= self.faces.len() {
            self.faces.resize(face_id.0 as usize + 1, [PointIndex(0); 3]);
        }
        self.faces[face_id.0 as usize] = face;
    }

    pub fn face(&self, face_id: FaceIndex) -> Face {
        self.faces[face_id.0 as usize]
    }

    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }

    pub fn set_num_faces(&mut self, num_faces: usize) {
        self.faces.resize(num_faces, [PointIndex(0); 3]);
    }
}

impl Deref for Mesh {
    type Target = PointCloud;

    fn deref(&self) -> &Self::Target {
        &self.point_cloud
    }
}

impl DerefMut for Mesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.point_cloud
    }
}
