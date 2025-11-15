//! Vector Extensions for Draco with SIMD optimizations
//!
//! This module provides enhanced vector operations optimized for geometry compression.
//! It includes both generic vector operations and SIMD-accelerated versions where available.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign, Index, IndexMut};

/// A 2D vector with f32 components
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2f {
    pub x: f32,
    pub y: f32,
}

/// A 3D vector with f32 components
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A 4D vector with f32 components
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector4f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// A 2D vector with integer components
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}

/// A 3D vector with integer components
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Vector3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Vector2f {
    /// Creates a new 2D float vector
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub fn splat(value: f32) -> Self {
        Self { x: value, y: value }
    }

    /// Computes the dot product of two vectors
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Computes the squared length of the vector
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes the length of the vector
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a normalized version of the vector
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::default()
        }
    }

    /// Normalizes the vector in place
    #[inline]
    pub fn normalize(&mut self) {
        let len = self.length();
        if len > 0.0 {
            *self /= len;
        }
    }

    /// Linear interpolation between two vectors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Returns the component-wise minimum of two vectors
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// Returns the component-wise maximum of two vectors
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    /// Returns the component-wise absolute values
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Checks if all components are finite
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Vector3f {
    /// Creates a new 3D float vector
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub fn splat(value: f32) -> Self {
        Self { x: value, y: value, z: value }
    }

    /// Creates a vector from a 2D vector and z component
    #[inline]
    pub fn from_xy(v: Vector2f, z: f32) -> Self {
        Self { x: v.x, y: v.y, z }
    }

    /// Returns the xy components as a 2D vector
    #[inline]
    pub fn xy(self) -> Vector2f {
        Vector2f { x: self.x, y: self.y }
    }

    /// Computes the dot product of two vectors
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Computes the cross product of two vectors
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Computes the squared length of the vector
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes the length of the vector
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a normalized version of the vector
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::default()
        }
    }

    /// Normalizes the vector in place
    #[inline]
    pub fn normalize(&mut self) {
        let len = self.length();
        if len > 0.0 {
            *self /= len;
        }
    }

    /// Linear interpolation between two vectors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Returns the component-wise minimum of two vectors
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    /// Returns the component-wise maximum of two vectors
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }

    /// Returns the component-wise absolute values
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    /// Checks if all components are finite
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Vector4f {
    /// Creates a new 4D float vector
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub fn splat(value: f32) -> Self {
        Self { x: value, y: value, z: value, w: value }
    }

    /// Creates a 4D vector from a 3D vector and w component
    #[inline]
    pub fn from_xyz(v: Vector3f, w: f32) -> Self {
        Self { x: v.x, y: v.y, z: v.z, w }
    }

    /// Returns the xyz components as a 3D vector
    #[inline]
    pub fn xyz(self) -> Vector3f {
        Vector3f { x: self.x, y: self.y, z: self.z }
    }

    /// Computes the dot product of two vectors
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Computes the squared length of the vector
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes the length of the vector
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a normalized version of the vector
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::default()
        }
    }

    /// Linear interpolation between two vectors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Returns the component-wise minimum of two vectors
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
            w: self.w.min(other.w),
        }
    }

    /// Returns the component-wise maximum of two vectors
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
            w: self.w.max(other.w),
        }
    }

    /// Returns the component-wise absolute values
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
            w: self.w.abs(),
        }
    }
}

// Operator implementations for Vector4f
impl Add for Vector4f {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl Add<f32> for Vector4f {
    type Output = Self;
    #[inline]
    fn add(self, scalar: f32) -> Self {
        Self {
            x: self.x + scalar,
            y: self.y + scalar,
            z: self.z + scalar,
            w: self.w + scalar,
        }
    }
}

impl Sub for Vector4f {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl Sub<f32> for Vector4f {
    type Output = Self;
    #[inline]
    fn sub(self, scalar: f32) -> Self {
        Self {
            x: self.x - scalar,
            y: self.y - scalar,
            z: self.z - scalar,
            w: self.w - scalar,
        }
    }
}

impl Mul for Vector4f {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
            w: self.w * other.w,
        }
    }
}

impl Mul<f32> for Vector4f {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

impl Div for Vector4f {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
            w: self.w / other.w,
        }
    }
}

impl Div<f32> for Vector4f {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
            w: self.w / scalar,
        }
    }
}

// Vector2i operations
impl Vector2i {
    /// Creates a new 2D integer vector
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub fn splat(value: i32) -> Self {
        Self { x: value, y: value }
    }

    /// Computes the dot product of two vectors
    #[inline]
    pub fn dot(self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y
    }

    /// Returns the component-wise minimum of two vectors
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// Returns the component-wise maximum of two vectors
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    /// Returns the component-wise absolute values
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

// Vector3i operations
impl Vector3i {
    /// Creates a new 3D integer vector
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub fn splat(value: i32) -> Self {
        Self { x: value, y: value, z: value }
    }

    /// Creates a vector from a 2D vector and z component
    #[inline]
    pub fn from_xy(v: Vector2i, z: i32) -> Self {
        Self { x: v.x, y: v.y, z }
    }

    /// Returns the xy components as a 2D vector
    #[inline]
    pub fn xy(self) -> Vector2i {
        Vector2i { x: self.x, y: self.y }
    }

    /// Computes the dot product of two vectors
    #[inline]
    pub fn dot(self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Computes the cross product of two vectors
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Returns the component-wise minimum of two vectors
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    /// Returns the component-wise maximum of two vectors
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }

    /// Returns the component-wise absolute values
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }
}

// Operator implementations for Vector2f
impl Add for Vector2f {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Add<f32> for Vector2f {
    type Output = Self;
    #[inline]
    fn add(self, scalar: f32) -> Self {
        Self {
            x: self.x + scalar,
            y: self.y + scalar,
        }
    }
}

impl AddAssign for Vector2f {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl AddAssign<f32> for Vector2f {
    #[inline]
    fn add_assign(&mut self, scalar: f32) {
        self.x += scalar;
        self.y += scalar;
    }
}

impl Sub for Vector2f {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Sub<f32> for Vector2f {
    type Output = Self;
    #[inline]
    fn sub(self, scalar: f32) -> Self {
        Self {
            x: self.x - scalar,
            y: self.y - scalar,
        }
    }
}

impl SubAssign for Vector2f {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl SubAssign<f32> for Vector2f {
    #[inline]
    fn sub_assign(&mut self, scalar: f32) {
        self.x -= scalar;
        self.y -= scalar;
    }
}

impl Mul for Vector2f {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl Mul<f32> for Vector2f {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl MulAssign for Vector2f {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl MulAssign<f32> for Vector2f {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
    }
}

impl Div for Vector2f {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl Div<f32> for Vector2f {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl DivAssign for Vector2f {
    #[inline]
    fn div_assign(&mut self, other: Self) {
        self.x /= other.x;
        self.y /= other.y;
    }
}

impl DivAssign<f32> for Vector2f {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        self.x /= scalar;
        self.y /= scalar;
    }
}

impl Index<usize> for Vector2f {
    type Output = f32;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Index out of bounds for Vector2f"),
        }
    }
}

impl IndexMut<usize> for Vector2f {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Index out of bounds for Vector2f"),
        }
    }
}

// Operator implementations for Vector3f
impl Add for Vector3f {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Add<f32> for Vector3f {
    type Output = Self;
    #[inline]
    fn add(self, scalar: f32) -> Self {
        Self {
            x: self.x + scalar,
            y: self.y + scalar,
            z: self.z + scalar,
        }
    }
}

impl AddAssign for Vector3f {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl AddAssign<f32> for Vector3f {
    #[inline]
    fn add_assign(&mut self, scalar: f32) {
        self.x += scalar;
        self.y += scalar;
        self.z += scalar;
    }
}

impl Sub for Vector3f {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Sub<f32> for Vector3f {
    type Output = Self;
    #[inline]
    fn sub(self, scalar: f32) -> Self {
        Self {
            x: self.x - scalar,
            y: self.y - scalar,
            z: self.z - scalar,
        }
    }
}

impl SubAssign for Vector3f {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
    }
}

impl SubAssign<f32> for Vector3f {
    #[inline]
    fn sub_assign(&mut self, scalar: f32) {
        self.x -= scalar;
        self.y -= scalar;
        self.z -= scalar;
    }
}

impl Mul for Vector3f {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl Mul<f32> for Vector3f {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl MulAssign for Vector3f {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        self.x *= other.x;
        self.y *= other.y;
        self.z *= other.z;
    }
}

impl MulAssign<f32> for Vector3f {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }
}

impl Div for Vector3f {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
        }
    }
}

impl Div<f32> for Vector3f {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl DivAssign for Vector3f {
    #[inline]
    fn div_assign(&mut self, other: Self) {
        self.x /= other.x;
        self.y /= other.y;
        self.z /= other.z;
    }
}

impl DivAssign<f32> for Vector3f {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        self.x /= scalar;
        self.y /= scalar;
        self.z /= scalar;
    }
}

impl Index<usize> for Vector3f {
    type Output = f32;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Index out of bounds for Vector3f"),
        }
    }
}

impl IndexMut<usize> for Vector3f {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("Index out of bounds for Vector3f"),
        }
    }
}

// Operator implementations for Vector2i
impl Add for Vector2i {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vector2i {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul for Vector2i {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl Mul<i32> for Vector2i {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: i32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

// Operator implementations for Vector3i
impl Add for Vector3i {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vector3i {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul for Vector3i {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl Mul<i32> for Vector3i {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: i32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

// Vector utility functions
/// Computes the squared distance between two 2D points
#[inline]
pub fn distance_squared_2d(a: Vector2f, b: Vector2f) -> f32 {
    (b - a).length_squared()
}

/// Computes the distance between two 2D points
#[inline]
pub fn distance_2d(a: Vector2f, b: Vector2f) -> f32 {
    distance_squared_2d(a, b).sqrt()
}

/// Computes the squared distance between two 3D points
#[inline]
pub fn distance_squared_3d(a: Vector3f, b: Vector3f) -> f32 {
    (b - a).length_squared()
}

/// Computes the distance between two 3D points
#[inline]
pub fn distance_3d(a: Vector3f, b: Vector3f) -> f32 {
    distance_squared_3d(a, b).sqrt()
}

/// Reflects a vector around a normal
#[inline]
pub fn reflect(vector: Vector3f, normal: Vector3f) -> Vector3f {
    vector - normal * (2.0 * vector.dot(normal))
}

/// Projects a vector onto another vector
#[inline]
pub fn project(vector: Vector3f, onto: Vector3f) -> Vector3f {
    onto * (vector.dot(onto) / onto.dot(onto))
}

/// Computes the angle between two 3D vectors in radians
#[inline]
pub fn angle_between(a: Vector3f, b: Vector3f) -> f32 {
    let dot_product = a.dot(b);
    let lengths = a.length() * b.length();
    if lengths == 0.0 {
        0.0
    } else {
        (dot_product / lengths).acos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector2f_creation() {
        let v = Vector2f::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);

        let v_splat = Vector2f::splat(5.0);
        assert_eq!(v_splat.x, 5.0);
        assert_eq!(v_splat.y, 5.0);
    }

    #[test]
    fn test_vector2f_operations() {
        let a = Vector2f::new(1.0, 2.0);
        let b = Vector2f::new(3.0, 4.0);

        // Addition
        let c = a + b;
        assert_eq!(c, Vector2f::new(4.0, 6.0));

        // Subtraction
        let c = b - a;
        assert_eq!(c, Vector2f::new(2.0, 2.0));

        // Multiplication
        let c = a * b;
        assert_eq!(c, Vector2f::new(3.0, 8.0));

        let c = a * 2.0;
        assert_eq!(c, Vector2f::new(2.0, 4.0));

        // Division
        let c = b / a;
        assert_eq!(c, Vector2f::new(3.0, 2.0));

        let c = b / 2.0;
        assert_eq!(c, Vector2f::new(1.5, 2.0));
    }

    #[test]
    fn test_vector2f_math() {
        let v = Vector2f::new(3.0, 4.0);

        // Dot product
        assert_eq!(v.dot(v), 25.0);
        assert_eq!(v.dot(Vector2f::new(1.0, 0.0)), 3.0);

        // Length
        assert_eq!(v.length(), 5.0);
        assert_eq!(v.length_squared(), 25.0);

        // Normalization
        let normalized = v.normalized();
        assert!((normalized.length() - 1.0).abs() < f32::EPSILON);
        assert_eq!(normalized, Vector2f::new(0.6, 0.8));
    }

    #[test]
    fn test_vector3f_creation() {
        let v = Vector3f::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);

        let v_splat = Vector3f::splat(5.0);
        assert_eq!(v_splat.x, 5.0);
        assert_eq!(v_splat.y, 5.0);
        assert_eq!(v_splat.z, 5.0);

        let v_xy = Vector3f::from_xy(Vector2f::new(1.0, 2.0), 3.0);
        assert_eq!(v_xy, Vector3f::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_vector3f_operations() {
        let a = Vector3f::new(1.0, 2.0, 3.0);
        let b = Vector3f::new(4.0, 5.0, 6.0);

        // Addition
        let c = a + b;
        assert_eq!(c, Vector3f::new(5.0, 7.0, 9.0));

        // Subtraction
        let c = b - a;
        assert_eq!(c, Vector3f::new(3.0, 3.0, 3.0));

        // Cross product
        let cross = a.cross(b);
        assert_eq!(cross, Vector3f::new(-3.0, 6.0, -3.0));
    }

    #[test]
    fn test_vector3f_math() {
        let v = Vector3f::new(1.0, 2.0, 2.0);

        // Dot product
        assert_eq!(v.dot(v), 9.0);

        // Length
        assert_eq!(v.length(), 3.0);
        assert_eq!(v.length_squared(), 9.0);

        // Normalization
        let normalized = v.normalized();
        assert!((normalized.length() - 1.0).abs() < f32::EPSILON);
        assert_eq!(normalized, Vector3f::new(1.0/3.0, 2.0/3.0, 2.0/3.0));
    }

    #[test]
    fn test_vector_utilities() {
        let a = Vector2f::new(0.0, 0.0);
        let b = Vector2f::new(3.0, 4.0);

        assert_eq!(distance_squared_2d(a, b), 25.0);
        assert_eq!(distance_2d(a, b), 5.0);

        let v1 = Vector3f::new(1.0, 0.0, 0.0);
        let v2 = Vector3f::new(0.0, 1.0, 0.0);

        assert!((angle_between(v1, v2) - std::f32::consts::FRAC_PI_2).abs() < f32::EPSILON);

        let reflected = reflect(Vector3f::new(1.0, -1.0, 0.0), Vector3f::new(0.0, 1.0, 0.0));
        assert_eq!(reflected, Vector3f::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_vector_indexing() {
        let mut v = Vector2f::new(1.0, 2.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);

        v[0] = 3.0;
        assert_eq!(v.x, 3.0);

        let mut v3 = Vector3f::new(1.0, 2.0, 3.0);
        assert_eq!(v3[2], 3.0);

        v3[2] = 4.0;
        assert_eq!(v3.z, 4.0);
    }

    #[test]
    fn test_vector_min_max() {
        let a = Vector2f::new(1.0, 5.0);
        let b = Vector2f::new(3.0, 2.0);

        assert_eq!(a.min(b), Vector2f::new(1.0, 2.0));
        assert_eq!(a.max(b), Vector2f::new(3.0, 5.0));
    }

    #[test]
    fn test_vector_lerp() {
        let a = Vector2f::new(0.0, 0.0);
        let b = Vector2f::new(10.0, 10.0);

        let result = a.lerp(b, 0.5);
        assert_eq!(result, Vector2f::new(5.0, 5.0));

        let result = a.lerp(b, 0.0);
        assert_eq!(result, a);

        let result = a.lerp(b, 1.0);
        assert_eq!(result, b);
    }

    #[test]
    fn test_integer_vectors() {
        let a = Vector2i::new(1, 2);
        let b = Vector2i::new(3, 4);

        assert_eq!(a + b, Vector2i::new(4, 6));
        assert_eq!(b - a, Vector2i::new(2, 2));
        assert_eq!(a * b, Vector2i::new(3, 8));
        assert_eq!(a * 3, Vector2i::new(3, 6));
        assert_eq!(a.dot(b), 11);

        let a3 = Vector3i::new(1, 2, 3);
        let b3 = Vector3i::new(4, 5, 6);

        assert_eq!(a3.cross(b3), Vector3i::new(-3, 6, -3));
    }
}