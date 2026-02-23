use crate::agent::Strategy;
use csv::Writer;
use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;

/// Handles exporting simulation data to a CSV file.
pub struct Exporter {
    pub file_path: String,
}

impl Exporter {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }

    /// Write the entire history time-series to a fresh CSV (overwrites any existing file).
    pub fn export_history(&self, history: &VecDeque<(usize, HashMap<Strategy, usize>)>) {
        if history.is_empty() {
            return;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_path)
            .unwrap();

        let mut wtr = Writer::from_writer(file);

        wtr.write_record(&[
            "Generation",
            "AlwaysCooperate",
            "AlwaysDefect",
            "TitForTat",
            "GrimTrigger",
            "Pavlov",
            "Random",
        ])
        .unwrap();

        for (gen, counts) in history {
            wtr.write_record(&[
                gen.to_string(),
                counts.get(&Strategy::AlwaysCooperate).unwrap_or(&0).to_string(),
                counts.get(&Strategy::AlwaysDefect).unwrap_or(&0).to_string(),
                counts.get(&Strategy::TitForTat).unwrap_or(&0).to_string(),
                counts.get(&Strategy::GrimTrigger).unwrap_or(&0).to_string(),
                counts.get(&Strategy::Pavlov).unwrap_or(&0).to_string(),
                counts.get(&Strategy::Random).unwrap_or(&0).to_string(),
            ])
            .unwrap();
        }

        wtr.flush().unwrap();
    }
}
