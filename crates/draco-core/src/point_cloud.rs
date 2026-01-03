use crate::geometry_attribute::{GeometryAttributeType, PointAttribute};

#[derive(Debug, Default, Clone)]
pub struct PointCloud {
    attributes: Vec<PointAttribute>,
    num_points: usize,
}

impl PointCloud {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_num_points(&mut self, num_points: usize) {
        self.num_points = num_points;
    }

    pub fn add_attribute(&mut self, mut attribute: PointAttribute) -> i32 {
        if self.num_points == 0 && attribute.size() > 0 {
            self.num_points = attribute.size();
        }
        // Assign unique id if not set?
        // In C++, it seems to just add it.
        // But we need to handle unique ids.
        // For now, just push.
        let id = self.attributes.len() as i32;
        attribute.set_unique_id(id as u32);
        self.attributes.push(attribute);
        id
    }

    pub fn num_attributes(&self) -> i32 {
        self.attributes.len() as i32
    }

    pub fn attribute(&self, att_id: i32) -> &PointAttribute {
        &self.attributes[att_id as usize]
    }

    pub fn attribute_mut(&mut self, att_id: i32) -> &mut PointAttribute {
        &mut self.attributes[att_id as usize]
    }

    pub fn named_attribute_id(&self, att_type: GeometryAttributeType) -> i32 {
        for (i, att) in self.attributes.iter().enumerate() {
            if att.attribute_type() == att_type {
                return i as i32;
            }
        }
        -1
    }

    pub fn named_attribute(&self, att_type: GeometryAttributeType) -> Option<&PointAttribute> {
        let id = self.named_attribute_id(att_type);
        if id >= 0 {
            Some(&self.attributes[id as usize])
        } else {
            None
        }
    }

    pub fn num_points(&self) -> usize {
        self.num_points
    }
}
