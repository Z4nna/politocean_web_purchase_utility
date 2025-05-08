use umya_spreadsheet::{writer, reader, Spreadsheet};
use std::io::Cursor;

use super::item;

#[derive(Debug, Clone)]
pub struct KiCadItem {
    pub quantity: i32,
    pub manifacturer: String,
    pub manifacturer_pn: String,
}

pub fn create_bom_file() -> Spreadsheet {
    let mut book: Spreadsheet = umya_spreadsheet::new_file();
    let _sheet = book.new_sheet("Ordine").unwrap();

    // Header row
    let order_sheet = book.get_sheet_by_name_mut("Ordine").unwrap();

    order_sheet.get_cell_mut((1,1)).set_value("Description");
    order_sheet.get_cell_mut((2,1)).set_value("Manufacturer PN");
    order_sheet.get_cell_mut((3,1)).set_value("Manufacturer");
    order_sheet.get_cell_mut((4,1)).set_value("Quantity");
    order_sheet.get_cell_mut((5,1)).set_value("Unit price");
    order_sheet.get_cell_mut((6,1)).set_value("Price");
    order_sheet.get_cell_mut((7,1)).set_value("Price incl. VAT");
    order_sheet.get_cell_mut((8,1)).set_value("Proposta (Descrizione spesa)");
    order_sheet.get_cell_mut((9,1)).set_value("Link");
    order_sheet.get_cell_mut((10,1)).set_value("Project");
    order_sheet.get_cell_mut((11,1)).set_value("Delivered");


    order_sheet.get_cell_mut((5,2)).set_value("Total:");
    order_sheet.get_cell_mut((6,2)).set_value("0,00");
    order_sheet.get_cell_mut((7,2)).set_value("0,00");

    book
}

pub fn add_item_to_bom(
    book: &mut Spreadsheet,
    manifacturer: String,
    manifacturer_pn: String,
    quantity: i32,
    description: String,
    unit_price: f64,
    proposal: String,
    link: String,
    project: String,
    delivered: String,
) -> Result<(), String> {
    let order_sheet = book.get_sheet_by_name_mut("Ordine");
    match order_sheet {
        Some(order_sheet) => {
            let row_index = order_sheet.get_highest_row();
            order_sheet.get_cell_mut((1, row_index)).set_value(description);
            order_sheet.get_cell_mut((2, row_index)).set_value(manifacturer_pn);
            order_sheet.get_cell_mut((3, row_index)).set_value(manifacturer);
            order_sheet.get_cell_mut((4, row_index)).set_value(quantity.to_string());
            order_sheet.get_cell_mut((5, row_index)).set_value(unit_price.to_string());
            order_sheet.get_cell_mut((6, row_index)).set_formula(&format!("=D{}*E{}", row_index, row_index));
            order_sheet.get_cell_mut((7, row_index)).set_formula(&format!("=F{}*1.22", row_index));
            order_sheet.get_cell_mut((8, row_index)).set_value(proposal);
            order_sheet.get_cell_mut((9, row_index)).set_value(link);
            order_sheet.get_cell_mut((10, row_index)).set_value(project);
            order_sheet.get_cell_mut((11, row_index)).set_value(delivered);

            order_sheet.get_cell_mut((5, row_index + 1)).set_value("Total:");
            order_sheet.get_cell_mut((6, row_index + 1)).set_formula(&format!("=SUM(F2:F{})", row_index));
            order_sheet.get_cell_mut((7, row_index + 1)).set_formula(&format!("=SUM(G2:G{})", row_index));

            Ok(())
        }
        None => {
            Err("Sheet not found".to_string())
        }
    }
}

pub fn save_to_bytes(book: &Spreadsheet) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    writer::xlsx::write_writer(book, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub fn load_from_bytes(bytes: &[u8]) -> Result<Spreadsheet, String> {
    let cursor = Cursor::new(bytes);
    let book = reader::xlsx::read_reader(cursor, true).map_err(|e| e.to_string())?;
    Ok(book)
}

pub fn save_to_file(book: &Spreadsheet) {
    let path = std::path::Path::new("/Users/michelecarenini/Desktop/test.xlsx");
    let _ = writer::xlsx::write(&book, path);
}

pub fn parse_kicad_bom_file(book: &Spreadsheet) -> Result<Vec<KiCadItem>, String> {
    let sheet = book.get_sheet(&0).expect("Sheet not found");
    let mut items: Vec<KiCadItem> = Vec::new();

    for row_num in 2..=sheet.get_highest_row() {
        println!("Row number: {}", row_num);
        let quantity = sheet.get_value((1, row_num)).parse::<i32>().unwrap_or(0);
        let manifacturer = sheet.get_value((2, row_num));
        let manifacturer_pn = sheet.get_value((3, row_num));
        println!("Read {} {} {}...", quantity, manifacturer, manifacturer_pn);
        if quantity > 0 && !manifacturer.is_empty() && !manifacturer_pn.is_empty() { 
            println!("Adding {} {} {}...", quantity, manifacturer, manifacturer_pn);
            items.push(KiCadItem {
                quantity,
                manifacturer,
                manifacturer_pn,
            });
        }
    }
    Ok(items)
}