use crate::agent::Strategy;
use crate::simulation::Simulation;
use csv::Writer;
use std::fs::OpenOptions;

/// Handles exporting simulation data to a CSV file.
pub struct Exporter {
    pub file_path: String,
    pub export_frequency: usize,
    initialized: bool,
}

impl Exporter {
    /// Creates a new exporter with the given file path and export frequency.
    pub fn new(file_path: String, export_frequency: usize) -> Self {
        Self {
            file_path,
            export_frequency,
            initialized: false,
        }
    }

    pub fn export_generation(&mut self, sim: &Simulation) {
        if sim.generation % self.export_frequency != 0 {
            return;
        }
        self.force_export(sim);
    }

    pub fn force_export(&mut self, sim: &Simulation) {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.file_path)
            .unwrap();

        let mut wtr = Writer::from_writer(file);
        
        if !self.initialized {
            // Write header
            wtr.write_record(&[
                "Generation",
                "AlwaysCooperate",
                "AlwaysDefect",
                "TitForTat",
                "GrimTrigger",
                "Pavlov",
                "Random",
            ]).unwrap();
            self.initialized = true;
        }

        let counts = sim.get_strategy_counts();
        
        wtr.write_record(&[
            sim.generation.to_string(),
            counts.get(&Strategy::AlwaysCooperate).unwrap_or(&0).to_string(),
            counts.get(&Strategy::AlwaysDefect).unwrap_or(&0).to_string(),
            counts.get(&Strategy::TitForTat).unwrap_or(&0).to_string(),
            counts.get(&Strategy::GrimTrigger).unwrap_or(&0).to_string(),
            counts.get(&Strategy::Pavlov).unwrap_or(&0).to_string(),
            counts.get(&Strategy::Random).unwrap_or(&0).to_string(),
        ]).unwrap();

        wtr.flush().unwrap();
    }
}
