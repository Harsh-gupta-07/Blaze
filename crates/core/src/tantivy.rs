use tantivy::{
    doc,
    schema::*,
    Index,
    IndexWriter,
    TantivyError,
    Term,
};

use crate::{db::app_data_dir, types::FileEntry};

pub struct TantivyState {
    pub writer: IndexWriter,
    pub id_field: Field,
    pub kind_field: Field,
    pub path_field: Field,
    pub name_field: Field,
}

pub fn initialize_index(
) -> tantivy::Result<TantivyState> {
    let tantivy_path = app_data_dir().join("db/tantivy");
    std::fs::create_dir_all(&tantivy_path)
        .unwrap();

    let schema = build_schema();

    let index_path = tantivy_path.to_str().unwrap_or(".db/tantivy").to_string();
    let index = open_or_create_index(
        &index_path,
        &schema,
    )?;

    println!(
        "[tantivy] initialized index at {}",
        index_path,
    );

    let schema = index.schema();
    let id_field = schema.get_field("id")?;
    let kind_field = schema.get_field("kind")?;
    let path_field = schema.get_field("path")?;
    let name_field = schema.get_field("name")?;

    let writer = index.writer(50_000_000)?;

    Ok(TantivyState {
        writer,
        id_field,
        kind_field,
        path_field,
        name_field,
    })
}

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    schema_builder.add_u64_field(
        "id",
        INDEXED | STORED,
    );

    schema_builder.add_text_field(
        "kind",
        STRING | STORED,
    );

    schema_builder.add_text_field(
        "path",
        STRING | STORED,
    );

    schema_builder.add_text_field(
        "name",
        TEXT | STORED,
    );

    schema_builder.build()
}

fn open_or_create_index(
    index_path: &str,
    schema: &Schema,
) -> tantivy::Result<Index> {
    match Index::open_in_dir(index_path) {
        Ok(index) if has_expected_fields(&index.schema()) => {
            Ok(index)
        }

        Ok(_) => {
            std::fs::remove_dir_all(index_path)
                .map_err(TantivyError::from)?;
            std::fs::create_dir_all(index_path)
                .map_err(TantivyError::from)?;
            Index::create_in_dir(
                index_path,
                schema.clone(),
            )
        }

        Err(_) => Index::create_in_dir(
            index_path,
            schema.clone(),
        ),
    }
}

fn has_expected_fields(schema: &Schema) -> bool {
    schema.get_field("id").is_ok()
        && schema.get_field("kind").is_ok()
        && schema.get_field("path").is_ok()
        && schema.get_field("name").is_ok()
}

pub fn make_index(files: &[FileEntry], tantivy: &mut TantivyState) -> tantivy::Result<()> {
    println!(
        "[tantivy] bulk indexing {} entries",
        files.len(),
    );

    for file in files{
        tantivy.writer.add_document(doc!(
            tantivy.id_field => file.id as u64,
            tantivy.kind_field => file.kind.as_str(),
            tantivy.path_field => file.path.as_str(),
            tantivy.name_field => file.name.as_str(),
        ))?;
    }

    tantivy.writer.commit()?;

    Ok(())
}


pub fn add_document(
    tantivy: &mut TantivyState,
    file: &FileEntry,
) -> tantivy::Result<()> {
    tantivy.writer.add_document(doc!(
        tantivy.id_field => file.id as u64,
        tantivy.kind_field => file.kind.as_str(),
        tantivy.path_field =>
            file.path.as_str(),
        tantivy.name_field =>
            file.name.as_str(),
    ))?;

    println!(
        "[tantivy] added document {}",
        file.path,
    );

    Ok(())
}

pub fn delete_document(
    tantivy: &mut TantivyState,
    path: &str,
) {
    tantivy.writer.delete_term(
        Term::from_field_text(
            tantivy.path_field,
            path,
        ),
    );

    println!(
        "[tantivy] deleted document {}",
        path,
    );
}

pub fn delete_documents(
    tantivy: &mut TantivyState,
    paths: &[String],
) {
    for path in paths {
        delete_document(
            tantivy,
            path,
        );
    }
}

pub fn update_document(
    tantivy: &mut TantivyState,
    file: &FileEntry,
) -> tantivy::Result<()> {
    delete_document(
        tantivy,
        &file.path,
    );

    add_document(
        tantivy,
        file,
    )?;

    println!(
        "[tantivy] updated document {}",
        file.path,
    );

    Ok(())
}

pub fn commit(
    tantivy: &mut TantivyState,
) -> tantivy::Result<()> {
    tantivy.writer.commit()?;

    println!("[tantivy] committed writer");

    Ok(())
}
