use tantivy::{
    doc,
    schema::*,
    Index,
};
use crate::types::FileEntry;

pub fn make_index(files: &[FileEntry]) -> tantivy::Result<()> {
    // -----------------------------
    // Schema
    // -----------------------------
    let mut schema_builder = Schema::builder();

    let path_field = schema_builder.add_text_field("path", TEXT | STORED);

    let name_field = schema_builder.add_text_field("name", TEXT | STORED);

    let schema = schema_builder.build();

    // -----------------------------
    // Create index
    // -----------------------------
    let index = Index::create_in_dir("./db/tantivy_index", schema.clone())?;

    // -----------------------------
    // Writer
    // -----------------------------
    let mut writer = index.writer(50_000_000)?;

    // Add documents
    for file in files{
        writer.add_document(doc!(
        path_field => file.path.as_str(),
        name_field => file.name.as_str(),
    ))?;
    }

    // Commit index
    writer.commit()?;

    Ok(())
}
