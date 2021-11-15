//! CSV output formatting for data.

use std::cell::Cell;
use std::collections::HashMap;
use std::fs::{DirBuilder, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{self, Instant, SystemTime};

use clockwise_common::output::DataWriter;
use clockwise_common::trace::SerialTrace;
use clockwise_common::test::{Execution, Response, Test};

struct Point {
    field: u8,
    t: Instant,
    raw: String,
}

#[derive(Debug)]
pub struct CSVDataWriter {
    base_path: PathBuf,
    columns: Cell<u8>,
}

impl CSVDataWriter {
    pub fn new(base_path: &Path) -> CSVDataWriter {
        let mut dir_builder = DirBuilder::new();
        dir_builder.recursive(true);
        dir_builder.create(base_path)
            .expect("could not create CSV data output directory");

        CSVDataWriter {
            base_path: PathBuf::from(base_path),
            columns: Cell::new(0),
        }
    }

    fn write_header(&self, writer: &mut dyn Write, columns: &[&str]) -> Result<(), String> {
        assert!(columns.len() > 0);
        assert!(columns.len() < 256);

        self.columns.set(columns.len() as u8);

        let mut header = String::new();
        for column in columns {
            header.push_str(column);
            header.push_str(",");
        }
        header.remove(header.len()-1);
        header.push('\n');

        writer.write(header.as_bytes())
            .map_err(|e| format!("failed to write header: {}", e))?;

        Ok(())
    }

    fn write_columns(&self, writer: &mut dyn Write, data: &[&str]) -> Result<(), String> {
        assert!(data.len() > 0);
        assert!(data.len() < 256);

        if data.len() != self.columns.get() as usize {
            Err(format!("inconsistent column count: {} (vs. {}", data.len(), self.columns.get()))
        } else {
            let mut row = String::new();
            for d in data {
                row.push_str(d);
                row.push_str(",");
            }
            row.remove(row.len()-1);
            row.push('\n');

            writer.write(row.as_bytes())
                .map_err(|e| format!("failed to write data row: {}", e))?;

            Ok(())
        }
    }
}

impl DataWriter for CSVDataWriter {
    fn save_output(&self,
                   test: &Test,
                   execution: &Execution,
                   responses: &[Response],
                   traces: &[SerialTrace],
                   energy: &HashMap<String, Vec<(Instant, f32)>>)
                   -> Result<(), String>
    {
        let csv_path = {
            let secs_epoch = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
            let file_name = format!("{}-{}.csv", test.get_id(), secs_epoch.as_secs());
            self.base_path.join(&file_name)
        };

        let mut csv_writer = {
            let file = File::create(&csv_path)
                .map_err(|e| format!("cannot open CSV ({}) for writing: {}", csv_path.display(), e))?;
            BufWriter::new(file)
        };

        let columns = vec!["time", "energy_mw"];
        self.write_header(&mut csv_writer, &columns)?;

        /* Coalescing data streams...
        - Sort them by their timestamps.
        - For the most part, only one stat changes at a time then; update all stats that change at that time.
        - Record their values at that state, 0 if not defined yet. */
        let mut points = Vec::new();
        let samples: &Vec<_> = energy.get("system").unwrap();
        for (t, val) in samples.iter().copied() {
            points.push(Point {
                field: 1,
                t,
                raw: format!("{:.4}", val),
            });
        }

        // get the number of fields
        let no_fields = points.iter()
            .map(|p| p.field)
            .max()
            .unwrap();

        let point_idx = 0;
        let mut row = vec![None; no_fields as usize + 1];
        let mut all_valid = false;
        // set all fields that have a valid initial value
        row[1] = Some("0".to_string());
        for point in points {
            // set the field specified by the point
            row[point.field as usize] = Some(point.raw);

            if !all_valid {
                // check that all the fields have a value
                // except skip the first field because it is the time which is always valid
                all_valid = (&row[1..]).into_iter()
                    .fold(true, |curr, row_state| {
                        curr && (row_state.is_some())
                    });
            } else {
                // update the timestamp
                let t = point.t - execution.get_start();
                row[0] = Some(format!("{}", t.as_micros()));
                // write the fields, we know they are all valid now
                let row_vals: Vec<_> = row.iter().map(|o| o.as_ref().unwrap().as_str()).collect();
                self.write_columns(&mut csv_writer, row_vals.as_slice())?;
            }
        }

        Ok(())
    }
}
