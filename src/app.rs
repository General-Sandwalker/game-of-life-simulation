use crate::agent::Strategy;
use crate::config::Config;
use crate::export::Exporter;
use crate::simulation::Simulation;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::collections::VecDeque;
use std::time::Instant;

/// The main application state and UI logic.
pub struct App {
    simulation: Simulation,
    is_playing: bool,
    steps_per_frame: usize,
    history: VecDeque<(usize, std::collections::HashMap<Strategy, usize>)>,
    // Visualization
    show_heatmap: bool,
    zoom: f32,
    pan: egui::Vec2,
    // Injection
    selected_strategy: Strategy,
    inject_radius: usize,
    // Export
    auto_export: bool,
    export_path: String,
    // Performance tracking
    last_step_time: Instant,
    steps_since_last_measure: usize,
    measured_gen_per_sec: f64,
}

impl App {
    /// Creates a new application instance with default configuration.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::default();
        let simulation = Simulation::new(config);
        Self {
            simulation,
            is_playing: false,
            steps_per_frame: 1,
            history: VecDeque::new(),
            show_heatmap: false,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            selected_strategy: Strategy::TitForTat,
            inject_radius: 1,
            auto_export: false,
            export_path: "export.csv".to_string(),
            last_step_time: Instant::now(),
            steps_since_last_measure: 0,
            measured_gen_per_sec: 0.0,
        }
    }

    fn reset_simulation(&mut self) {
        self.simulation = Simulation::new(self.simulation.config.clone());
        self.history.clear();
        self.steps_since_last_measure = 0;
        self.measured_gen_per_sec = 0.0;
        self.last_step_time = Instant::now();
    }

    fn inject_strategy_at(&mut self, cell_x: usize, cell_y: usize) {
        let w = self.simulation.config.grid_width;
        let h = self.simulation.config.grid_height;
        let r = self.inject_radius as isize;
        let strategy = self.selected_strategy;

        for dy in -r..=r {
            for dx in -r..=r {
                let nx = cell_x as isize + dx;
                let ny = cell_y as isize + dy;
                if nx >= 0 && nx < w as isize && ny >= 0 && ny < h as isize {
                    let idx = (ny as usize) * w + (nx as usize);
                    self.simulation.agents[idx].strategy = strategy;
                    self.simulation.agents[idx].history.clear();
                    self.simulation.agents[idx].payoff = 0.0;
                    self.simulation.agents[idx].age = 0;
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Simulation tick ───────────────────────────────────────────────────
        if self.is_playing {
            for _ in 0..self.steps_per_frame {
                if self.simulation.generation >= self.simulation.config.max_generations {
                    self.is_playing = false;
                    if self.auto_export {
                        Exporter::new(self.export_path.clone()).export_history(&self.history);
                    }
                    break;
                }

                self.simulation.step();
                self.steps_since_last_measure += 1;

                self.history.push_back((
                    self.simulation.generation,
                    self.simulation.get_strategy_counts(),
                ));
            }

            // Measure generations per second every 500 ms
            let elapsed = self.last_step_time.elapsed().as_secs_f64();
            if elapsed >= 0.5 {
                self.measured_gen_per_sec = self.steps_since_last_measure as f64 / elapsed;
                self.steps_since_last_measure = 0;
                self.last_step_time = Instant::now();
            }

            ctx.request_repaint();
        }

        // ── Side Panel ────────────────────────────────────────────────────────
        egui::SidePanel::left("controls").min_width(240.0).show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.heading("Prisoner's Dilemma");
                    ui.separator();

                    // Playback controls
                    ui.horizontal(|ui| {
                        let lbl = if self.is_playing { "⏸ Pause" } else { "▶ Play" };
                        if ui.button(lbl).clicked() {
                            self.is_playing = !self.is_playing;
                            self.last_step_time = Instant::now();
                            self.steps_since_last_measure = 0;
                        }
                        if ui.button("⏭ Step").clicked() {
                            self.simulation.step();
                            self.history.push_back((
                                self.simulation.generation,
                                self.simulation.get_strategy_counts(),
                            ));
                        }
                        if ui.button("↺ Reset").clicked() {
                            self.reset_simulation();
                        }
                    });

                    ui.add(
                        egui::Slider::new(&mut self.steps_per_frame, 1..=200)
                            .text("Speed (gen/frame)"),
                    );

                    ui.separator();

                    // Grid & Population
                    ui.collapsing("Grid & Population", |ui| {
                        let mut changed = false;
                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            changed |= ui
                                .add(egui::DragValue::new(&mut self.simulation.config.grid_width).range(5..=500))
                                .changed();
                            ui.label("Height:");
                            changed |= ui
                                .add(egui::DragValue::new(&mut self.simulation.config.grid_height).range(5..=500))
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Max generations:");
                            changed |= ui
                                .add(egui::DragValue::new(&mut self.simulation.config.max_generations))
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Random seed:");
                            changed |= ui
                                .add(egui::DragValue::new(&mut self.simulation.config.seed))
                                .changed();
                        });
                        if changed {
                            self.reset_simulation();
                        }
                    });

                    // Evolution parameters
                    ui.collapsing("Evolution", |ui| {
                        ui.add(
                            egui::Slider::new(
                                &mut self.simulation.config.neighborhood_n,
                                0.0..=1.0,
                            )
                            .text("Locality (N)"),
                        );
                        ui.label("  0=random pairing, 1=local only");
                        ui.add(
                            egui::Slider::new(
                                &mut self.simulation.config.mutation_rate,
                                0.0..=0.1,
                            )
                            .text("Mutation rate"),
                        );
                    });

                    // Payoff matrix
                    ui.collapsing("Payoff Matrix", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.simulation.config.payoff_t, 0.0..=10.0)
                                .text("T (Temptation)"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.simulation.config.payoff_r, 0.0..=10.0)
                                .text("R (Reward)"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.simulation.config.payoff_p, 0.0..=10.0)
                                .text("P (Punishment)"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.simulation.config.payoff_s, 0.0..=10.0)
                                .text("S (Sucker)"),
                        );
                        if let Some(err) = self.simulation.validate_payoffs() {
                            ui.colored_label(egui::Color32::RED, format!("⚠ {}", err));
                        } else {
                            ui.colored_label(egui::Color32::GREEN, "✓ Valid PD payoffs");
                        }
                    });

                    // Presets
                    ui.collapsing("Presets", |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("💾 Save preset").clicked() {
                                if let Ok(json) =
                                    serde_json::to_string_pretty(&self.simulation.config)
                                {
                                    let _ = std::fs::write("preset.json", json);
                                }
                            }
                            if ui.button("📂 Load preset").clicked() {
                                if let Ok(json) = std::fs::read_to_string("preset.json") {
                                    if let Ok(config) = serde_json::from_str::<Config>(&json) {
                                        self.simulation.config = config;
                                        self.reset_simulation();
                                    }
                                }
                            }
                        });
                    });

                    // Export
                    ui.collapsing("Export", |ui| {
                        ui.checkbox(&mut self.auto_export, "Auto-export when run ends");
                        ui.label("CSV file:");
                        ui.text_edit_singleline(&mut self.export_path);
                        let tip = format!(
                            "Export {} recorded generations to CSV",
                            self.history.len()
                        );
                        if ui
                            .add_enabled(
                                !self.history.is_empty(),
                                egui::Button::new("⬇ Export history"),
                            )
                            .on_hover_text(tip)
                            .clicked()
                        {
                            let e = Exporter::new(self.export_path.clone());
                            e.export_history(&self.history);
                        }
                    });

                    // Visualization
                    ui.collapsing("Visualization", |ui| {
                        ui.checkbox(&mut self.show_heatmap, "Payoff heatmap overlay");
                        ui.label("Double-click grid to reset zoom/pan.");
                    });

                    // Inject strategies
                    ui.collapsing("Inject Strategy", |ui| {
                        egui::ComboBox::from_label("Strategy")
                            .selected_text(format!("{:?}", self.selected_strategy))
                            .show_ui(ui, |ui| {
                                for s in Strategy::all() {
                                    ui.selectable_value(
                                        &mut self.selected_strategy,
                                        s,
                                        format!("{:?}", s),
                                    );
                                }
                            });
                        ui.add(
                            egui::Slider::new(&mut self.inject_radius, 0..=20)
                                .text("Brush radius"),
                        );
                        ui.label("Right-click on grid to paint.");
                    });

                    ui.separator();

                    // Statistics
                    ui.heading("Statistics");
                    ui.label(format!("Generation:  {}", self.simulation.generation));
                    ui.label(format!(
                        "Gen/sec:     {:.1}",
                        if self.is_playing { self.measured_gen_per_sec } else { 0.0 }
                    ));
                    ui.label(format!(
                        "Cooperation: {:.1}%",
                        self.simulation.get_cooperation_rate() * 100.0
                    ));
                    ui.label(format!("Population:  {}", self.simulation.agents.len()));

                    ui.add_space(4.0);
                    ui.label("Strategy  |  count  |  %  |  avg payoff");

                    let total = self.simulation.agents.len() as f32;
                    let counts = self.simulation.get_strategy_counts();
                    let avg_payoffs = self.simulation.get_avg_payoffs_by_strategy();

                    for strategy in Strategy::all() {
                        let count = *counts.get(&strategy).unwrap_or(&0);
                        let avg_p = avg_payoffs.get(&strategy).copied().unwrap_or(0.0);
                        let pct = if total > 0.0 {
                            count as f32 / total * 100.0
                        } else {
                            0.0
                        };
                        let c = strategy.color();
                        ui.horizontal(|ui| {
                            let (dot, _) = ui.allocate_exact_size(
                                egui::vec2(12.0, 12.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                dot,
                                2.0,
                                egui::Color32::from_rgb(c[0], c[1], c[2]),
                            );
                            ui.label(format!(
                                "{:?}: {} ({:.1}%) {:.2}",
                                strategy, count, pct, avg_p
                            ));
                        });
                    }
                });
        });

        // ── Central Panel ─────────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let grid_height = available.y * 0.65;
            let plot_height = (available.y - grid_height - 30.0).max(50.0);

            // Grid viewport
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(available.x, grid_height),
                egui::Sense::click_and_drag(),
            );

            // Left-drag to pan
            if response.dragged_by(egui::PointerButton::Primary) {
                self.pan += response.drag_delta();
            }

            // Scroll to zoom
            if response.hovered() {
                let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let zoom_delta = (scroll * 0.002).exp();
                    self.zoom = (self.zoom * zoom_delta).clamp(0.05, 200.0);
                }
            }

            // Double-click resets view
            if response.double_clicked() {
                self.zoom = 1.0;
                self.pan = egui::Vec2::ZERO;
            }

            let cell_w = (rect.width() / self.simulation.config.grid_width as f32) * self.zoom;
            let cell_h = (rect.height() / self.simulation.config.grid_height as f32) * self.zoom;

            // Right-click/drag to inject strategy
            if response.clicked_by(egui::PointerButton::Secondary)
                || response.dragged_by(egui::PointerButton::Secondary)
            {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = pos - rect.min - self.pan;
                    let cx = (local.x / cell_w).floor() as isize;
                    let cy = (local.y / cell_h).floor() as isize;
                    if cx >= 0
                        && cx < self.simulation.config.grid_width as isize
                        && cy >= 0
                        && cy < self.simulation.config.grid_height as isize
                    {
                        self.inject_strategy_at(cx as usize, cy as usize);
                    }
                }
            }

            // Background
            ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);

            let max_payoff = self
                .simulation
                .agents
                .iter()
                .map(|a| a.payoff)
                .fold(0.0_f32, f32::max);

            // Draw cells
            let mut shapes = Vec::with_capacity(self.simulation.agents.len());
            for i in 0..self.simulation.agents.len() {
                let x = i % self.simulation.config.grid_width;
                let y = i / self.simulation.config.grid_width;

                let cell_rect = egui::Rect::from_min_size(
                    rect.min + self.pan + egui::vec2(x as f32 * cell_w, y as f32 * cell_h),
                    egui::vec2(cell_w.max(1.0), cell_h.max(1.0)),
                );

                if !rect.intersects(cell_rect) {
                    continue;
                }

                let agent = &self.simulation.agents[i];
                let color = if self.show_heatmap {
                    let t = if max_payoff > 0.0 {
                        (agent.payoff / max_payoff).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    egui::Color32::from_rgb(
                        (t * 255.0) as u8,
                        0,
                        ((1.0 - t) * 200.0) as u8,
                    )
                } else {
                    let c = agent.strategy.color();
                    egui::Color32::from_rgb(c[0], c[1], c[2])
                };

                shapes.push(egui::Shape::rect_filled(cell_rect, 0.0, color));
            }
            ui.painter().with_clip_rect(rect).extend(shapes);

            // Hover tooltip
            if let Some(hover_pos) = response.hover_pos() {
                let local = hover_pos - rect.min - self.pan;
                let cx = (local.x / cell_w).floor() as isize;
                let cy = (local.y / cell_h).floor() as isize;
                if cx >= 0
                    && cx < self.simulation.config.grid_width as isize
                    && cy >= 0
                    && cy < self.simulation.config.grid_height as isize
                {
                    let idx = cy as usize * self.simulation.config.grid_width + cx as usize;
                    let agent = &self.simulation.agents[idx];
                    egui::show_tooltip_at_pointer(
                        ctx,
                        egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("tt_layer")),
                        egui::Id::new("cell_tooltip"),
                        |ui: &mut egui::Ui| {
                            ui.label(format!("Strategy: {:?}", agent.strategy));
                            ui.label(format!("Payoff:   {:.2}", agent.payoff));
                            ui.label(format!("Age:      {}", agent.age));
                            ui.label(format!("Memories: {}", agent.history.len()));
                        },
                    );
                }
            }

            ui.add_space(4.0);

            // Strategy distribution over time
            ui.label("Strategy distribution over time:");
            let plot = Plot::new("strategy_plot")
                .legend(Legend::default())
                .height(plot_height);

            plot.show(ui, |plot_ui| {
                for strategy in Strategy::all() {
                    let points: Vec<[f64; 2]> = self
                        .history
                        .iter()
                        .map(|(gen, counts)| {
                            [*gen as f64, *counts.get(&strategy).unwrap_or(&0) as f64]
                        })
                        .collect();

                    let c = strategy.color();
                    let line = Line::new(PlotPoints::new(points))
                        .name(format!("{:?}", strategy))
                        .color(egui::Color32::from_rgb(c[0], c[1], c[2]));

                    plot_ui.line(line);
                }
            });
        });
    }
}
