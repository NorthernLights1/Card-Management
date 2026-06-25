//! Spreadsheet import (xlsx / xls / csv). Parsing lives here so the UI only does
//! column mapping. Card numbers from the file are preserved.

use crate::patient::{ImportItem, PatientInput};
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

#[derive(serde::Serialize)]
pub struct ImportPreview {
    pub headers: Vec<String>,
    pub sample: Vec<Vec<String>>,
    pub total_rows: usize,
}

/// Column indices (0-based) chosen by the user in the mapping step.
#[derive(serde::Deserialize)]
pub struct Mapping {
    pub card_number: usize,
    pub first_name: usize,
    pub father_name: usize,
    pub grandfather_name: usize,
    pub sex: usize,
    pub phone: usize,
    pub age: Option<usize>,
    pub city: Option<usize>,
    pub address: Option<usize>,
}

pub fn preview(path: &Path) -> Result<ImportPreview, String> {
    let rows = read_rows(path)?;
    if rows.is_empty() {
        return Err("The file is empty".into());
    }
    let headers = rows[0].clone();
    let sample = rows.iter().skip(1).take(5).cloned().collect();
    Ok(ImportPreview {
        headers,
        sample,
        total_rows: rows.len().saturating_sub(1),
    })
}

pub fn build_items(path: &Path, m: &Mapping) -> Result<Vec<ImportItem>, String> {
    let rows = read_rows(path)?;
    let mut items = Vec::new();
    for (i, row) in rows.iter().enumerate().skip(1) {
        let cell = |idx: usize| row.get(idx).map(|s| s.trim().to_string()).unwrap_or_default();
        let opt_cell = |o: Option<usize>| {
            o.and_then(|idx| row.get(idx))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let age = opt_cell(m.age).and_then(|s| s.parse::<i64>().ok());
        let input = PatientInput {
            first_name: cell(m.first_name),
            father_name: cell(m.father_name),
            grandfather_name: cell(m.grandfather_name),
            sex: normalize_sex(&cell(m.sex)),
            phone: cell(m.phone),
            dob: None, // legacy import is age-based; DOB can be added later by editing
            age,
            address: opt_cell(m.address),
            city: opt_cell(m.city),
        };
        items.push(ImportItem {
            row_index: i + 1, // 1-based, header is row 1
            card_number: cell(m.card_number),
            input,
        });
    }
    Ok(items)
}

fn normalize_sex(s: &str) -> String {
    match s.trim().to_lowercase().as_str() {
        "m" | "male" => "Male".to_string(),
        "f" | "female" => "Female".to_string(),
        other => other.to_string(), // anything else fails validation and is reported
    }
}

fn read_rows(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "csv" {
        read_csv(path)
    } else {
        read_spreadsheet(path)
    }
}

fn read_csv(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)
        .map_err(e2s)?;
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(e2s)?;
        rows.push(rec.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

fn read_spreadsheet(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut wb = open_workbook_auto(path).map_err(e2s)?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or("The file has no sheets")?
        .map_err(e2s)?;
    Ok(range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect())
        .collect())
}

fn cell_to_string(c: &Data) -> String {
    match c {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn e2s<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}
