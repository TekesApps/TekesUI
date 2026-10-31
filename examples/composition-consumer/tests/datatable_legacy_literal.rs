use tekes_ui_controls::extended::DataTable;
#[test]
fn original_external_literal_destructure_and_constructor_compile() {
    let table = DataTable {
        selected_row: None,
        sort: None,
        rows: 2,
        columns: 1,
    };
    let DataTable {
        selected_row,
        sort,
        rows,
        columns,
    } = table;
    assert_eq!(
        DataTable::new(rows, columns),
        DataTable {
            selected_row,
            sort,
            rows,
            columns
        }
    );
}
