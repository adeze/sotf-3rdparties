// Rust guideline compliant 2026-02-21
#[path = "../common/glam_isometry.rs"]
mod glam_isometry;
#[path = "../common/glam_matrix.rs"]
mod glam_matrix;
#[path = "../common/glam_point.rs"]
mod glam_point;
#[path = "../common/glam_quaternion.rs"]
mod glam_quaternion;
#[path = "../common/glam_rotation.rs"]
mod glam_rotation;
#[path = "../common/glam_similarity.rs"]
mod glam_similarity;
#[path = "../common/glam_translation.rs"]
mod glam_translation;
#[path = "../common/glam_unit_complex.rs"]
mod glam_unit_complex;

use glam034 as glam;

#[cfg(test)]
mod tests {
    use super::glam;

    #[test]
    fn matrix_conversion_preserves_column_order_and_transformation() {
        let matrix = glam::Mat3::from_cols_array(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0]);
        let vector = glam::Vec3::new(2.0, -1.0, 3.0);
        let converted: crate::Matrix3<f32> = matrix.into();
        assert_eq!(converted[(0, 1)], 4.0);
        let transformed: glam::Vec3 = (converted * crate::Vector3::from(vector)).into();
        assert_eq!(transformed, matrix * vector);
        assert_eq!(glam::Mat3::from(converted), matrix);
    }

    #[test]
    fn vector_conversion_preserves_precision_and_signed_components() {
        let double = glam::DVec3::new(1.0 + f64::EPSILON, -2.5, 3.75);
        assert_eq!(glam::DVec3::from(crate::Vector3::from(double)), double);
        let signed = glam::IVec3::new(-2, 3, i32::MIN);
        assert_eq!(glam::IVec3::from(crate::Vector3::from(signed)), signed);
        let unsigned = glam::UVec3::new(1, 2, u32::MAX);
        assert_eq!(glam::UVec3::from(crate::Vector3::from(unsigned)), unsigned);
    }
}
