//! Shared OpenAPI representations for raw HTTP bodies rather than JSON values.

/// A raw binary body, represented as a browser Blob in generated contracts.
pub struct BinaryBody;

impl utoipa::PartialSchema for BinaryBody {
    /// Describe an uninterpreted binary HTTP entity, rather than a JSON string or byte array.
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{KnownFormat, ObjectBuilder, SchemaFormat, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Binary)))
            .into()
    }
}
impl utoipa::ToSchema for BinaryBody {}
