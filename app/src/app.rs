use std::path::PathBuf;

use eframe::egui;

use crate::audio::Audio;
use crate::config;
use crate::menus;
use crate::model::{couleur_defaut, Corde, FormeOnde, HarpeConfig, NOMBRE_CORDES};
use crate::theme::{self, Theme};

const FOND_APERCU: egui::Color32 = egui::Color32::from_rgb(60, 60, 60);

/// État principal de l'application et logique des écrans.
pub struct HarpeApp {
    config: HarpeConfig,
    corde_selectionnee: usize,
    fichier: Option<PathBuf>,
    modifie: bool,
    message: Option<String>,
    audio: Option<Audio>,
    erreur_audio: Option<String>,
    theme: Theme,
    accent: egui::Color32,
    panneau_gauche: bool,
    volumes_lies: bool,
    picker_agrandi: bool,
}

impl HarpeApp {
    /// Crée l'application : applique le thème initial, installe les menus
    /// natifs macOS et initialise la sortie audio.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::Sombre;
        let accent = theme::accent(theme);
        theme::appliquer(theme, &cc.egui_ctx);
        menus::installer(&cc.egui_ctx);

        let mut audio = None;
        let mut erreur_audio = None;
        match Audio::new() {
            Ok(a) => audio = Some(a),
            Err(e) => erreur_audio = Some(e),
        }

        HarpeApp {
            config: HarpeConfig::default(),
            corde_selectionnee: 0,
            fichier: None,
            modifie: false,
            message: None,
            audio,
            erreur_audio,
            theme,
            accent,
            panneau_gauche: false,
            volumes_lies: false,
            picker_agrandi: false,
        }
    }

    /// Applique un thème visuel à egui et synchronise la coche du menu natif.
    fn definir_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.theme = theme;
        self.accent = theme::accent(theme);
        theme::appliquer(theme, ctx);
        menus::mettre_a_jour_theme(theme);
    }

    /// Déplace le panneau de réglages à gauche ou à droite et met à jour le
    /// libellé de l'élément de menu correspondant.
    fn basculer_panneau(&mut self) {
        self.panneau_gauche = !self.panneau_gauche;
        menus::mettre_a_jour_panneau(self.panneau_gauche);
    }

    /// Consomme la file des actions déclenchées par le menu natif et les
    /// applique à l'état de l'application.
    fn traiter_actions_menu(&mut self, ctx: &egui::Context) {
        for action in menus::tirer_actions() {
            match action {
                menus::ActionMenu::Nouveau => {
                    self.config = HarpeConfig::default();
                    self.corde_selectionnee = 0;
                    self.fichier = None;
                    self.modifie = true;
                    self.message = Some("Nouvelle configuration créée.".to_string());
                }
                menus::ActionMenu::Ouvrir => self.ouvrir(),
                menus::ActionMenu::Enregistrer => self.enregistrer(),
                menus::ActionMenu::EnregistrerSous => self.enregistrer_sous(None),
                menus::ActionMenu::BasculerPanneau => self.basculer_panneau(),
                menus::ActionMenu::Theme(t) => self.definir_theme(t, ctx),
                menus::ActionMenu::Quitter => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                }
            }
        }
    }

    /// Joue la corde donnée si elle existe et est active.
    fn jouer_corde(&self, index: usize) {
        let Some(corde) = self.config.cordes.get(index) else {
            return;
        };
        if !corde.active {
            return;
        }
        if let Some(audio) = &self.audio {
            audio.jouer(corde.frequence, &corde.reglage);
        }
    }

    /// Ouvre un fichier de configuration via une boîte de dialogue.
    fn ouvrir(&mut self) {
        let Some(chemin) = rfd::FileDialog::new()
            .add_filter("Configuration harpe", &["harpcfg", "json"])
            .set_title("Ouvrir une configuration")
            .pick_file()
        else {
            return;
        };
        match config::charger(&chemin) {
            Ok(mut cfg) => {
                for corde in &mut cfg.cordes {
                    corde.rafraichir_note();
                }
                self.config = cfg;
                self.fichier = Some(chemin.clone());
                self.modifie = false;
                self.corde_selectionnee = self.corde_selectionnee.min(self.config.cordes.len() - 1);
                self.message = Some(format!("Configuration ouverte : {}", chemin.display()));
            }
            Err(e) => self.message = Some(format!("Erreur à l'ouverture : {e}")),
        }
    }

    /// Enregistre dans le fichier courant, ou demande un chemin s'il n'y en a
    /// pas encore été choisi.
    fn enregistrer(&mut self) {
        if let Some(chemin) = self.fichier.clone() {
            self.enregistrer_sous(Some(chemin));
        } else {
            self.enregistrer_sous(None);
        }
    }

    /// Enregistre la configuration dans un fichier (« Enregistrer sous ») et
    /// mémorise le chemin choisi.
    fn enregistrer_sous(&mut self, chemin: Option<PathBuf>) {
        let chemin = chemin
            .map(|c| config::avec_extension(&c))
            .or_else(|| {
                rfd::FileDialog::new()
                    .add_filter("Configuration harpe", &[config::EXTENSION])
                    .set_file_name(format!("harpe_config.{}", config::EXTENSION))
                    .set_title("Enregistrer la configuration")
                    .save_file()
            });
        let Some(chemin) = chemin else {
            return;
        };
        match config::sauvegarder(&chemin, &self.config) {
            Ok(()) => {
                self.fichier = Some(chemin.clone());
                self.modifie = false;
                self.message = Some(format!("Configuration enregistrée : {}", chemin.display()));
            }
            Err(e) => self.message = Some(format!("Erreur à l'enregistrement : {e}")),
        }
    }

    /// Barre supérieure : titre, actions Fichier, lecture d'un arpège, choix du
    /// thème et position du panneau.
    fn barre_outils(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Harpe sans corde")
                    .size(20.0)
                    .strong()
                    .color(self.accent),
            );
            ui.label(egui::RichText::new("Réglage des cordes").weak());
            ui.separator();
            if ui.button("Nouveau").clicked() {
                self.config = HarpeConfig::default();
                self.corde_selectionnee = 0;
                self.fichier = None;
                self.modifie = true;
                self.message = Some("Nouvelle configuration créée.".to_string());
            }
            let mut action_fichier: Option<menus::ActionMenu> = None;
            ui.menu_button("Fichier…", |ui| {
                for (libelle, action) in [
                    ("Ouvrir…", menus::ActionMenu::Ouvrir),
                    ("Enregistrer", menus::ActionMenu::Enregistrer),
                    ("Enregistrer sous…", menus::ActionMenu::EnregistrerSous),
                ] {
                    if ui.button(libelle).clicked() {
                        action_fichier = Some(action);
                        ui.close();
                    }
                }
            });
            match action_fichier {
                Some(menus::ActionMenu::Ouvrir) => self.ouvrir(),
                Some(menus::ActionMenu::Enregistrer) => self.enregistrer(),
                Some(menus::ActionMenu::EnregistrerSous) => self.enregistrer_sous(None),
                _ => {}
            }
            ui.separator();
            if ui
                .button(egui::RichText::new("▶ Jouer un harpège").color(self.accent))
                .on_hover_text("Joue toutes les cordes")
                .clicked()
            {
                if let Some(audio) = &self.audio {
                    audio.jouer_arpege(&self.config.cordes);
                }
            }
            ui.separator();
            let avant_theme = self.theme;
            egui::ComboBox::from_id_salt("choix_theme")
                .selected_text(self.theme.label())
                .show_ui(ui, |ui| {
                    for t in Theme::TOUTES {
                        ui.selectable_value(&mut self.theme, t, t.label());
                    }
                });
            if self.theme != avant_theme {
                self.definir_theme(self.theme, ui.ctx());
            }
            if ui
                .button(if self.panneau_gauche {
                    "Panneau à gauche"
                } else {
                    "Panneau à droite"
                })
                .on_hover_text("Déplace le panneau de réglages à gauche ou à droite")
                .clicked()
            {
                self.basculer_panneau();
            }
        });
    }

    /// Zone principale : les 22 cordes cliquables (couleur, note, fréquence).
    fn zone_harpe(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Les 22 cordes")
                .size(16.0)
                .strong()
                .color(self.accent),
        );
        ui.label(
            egui::RichText::new(
                "Cliquez sur une corde pour l'écouter, vous pouvez la configurer dans le panneau de configuration.",
            )
            .weak(),
        );
        ui.add_space(12.0);

        let largeur = 34.0;
        let hauteur = 150.0;
        let hauteur_zone = hauteur + 46.0;

        egui::ScrollArea::horizontal()
            .id_salt("zone_cordes")
            .show(ui, |ui| {
                ui.allocate_ui(egui::vec2(largeur * NOMBRE_CORDES as f32 + 80.0, hauteur_zone), |ui| {
                    ui.horizontal(|ui| {
                        for (i, corde) in self.config.cordes.iter().enumerate() {
                            let (rect, reponse) = ui
                                .vertical(|ui| {
                                    ui.set_min_width(largeur);
                                    let (rect, reponse) = ui.allocate_exact_size(
                                        egui::vec2(largeur, hauteur),
                                        egui::Sense::click(),
                                    );
                                    let mut couleur = egui::Color32::from_rgb(
                                        corde.couleur.unwrap_or_else(|| couleur_defaut(i))[0],
                                        corde.couleur.unwrap_or_else(|| couleur_defaut(i))[1],
                                        corde.couleur.unwrap_or_else(|| couleur_defaut(i))[2],
                                    );
                                    if !corde.active {
                                        couleur = couleur.gamma_multiply(0.35);
                                    }
                                    if reponse.hovered() {
                                        couleur = couleur.gamma_multiply(1.35);
                                    }
                                    let arrondi = egui::CornerRadius::same(5);
                                    let painter = ui.painter();
                                    painter.rect_filled(rect, arrondi, couleur);
                                    if self.corde_selectionnee == i {
                                        painter.rect_stroke(
                                            rect.expand(1.5),
                                            arrondi,
                                            egui::Stroke::new(2.0, self.accent),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(format!("{}", i + 1))
                                            .small()
                                            .weak(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&corde.note).strong().color(self.accent),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.1} Hz", corde.frequence))
                                            .small()
                                            .weak(),
                                    );
                                    (rect, reponse)
                                })
                                .inner;
                            if reponse.clicked() {
                                self.corde_selectionnee = i;
                                self.jouer_corde(i);
                            }
                            let _ = rect;
                            ui.add_space(6.0);
                        }
                    });
                });
            });
    }

    /// Vue d'ensemble en tableau : sélection et réglage rapide de chaque corde.
    fn tableau_cordes(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Vue d'ensemble")
                    .size(16.0)
                    .strong()
                    .color(self.accent),
            ); 
            // Bouton pour lier ou délier les volumes des cordes : si liés, le réglage d'une corde s'applique à toutes les autres
            let actif = self.volumes_lies;
            let bouton = if actif {
                ui.button(egui::RichText::new("Volumes liés").color(self.accent))
            } else {
                ui.button("Volumes indépendants")
            }
            .on_hover_text(
                "Régler le volume d'une corde applique le même changement à toutes les cordes en même temps.",
            );
            if bouton.clicked() {
                self.volumes_lies = !actif;

            }
        });
        ui.add_space(6.0);
        let hauteur_tableau = ui.available_height().max(200.0);
        let mut changement_volume: Option<(usize, f32)> = None;
        egui::ScrollArea::vertical()
            .id_salt("tableau_cordes")
            .auto_shrink([false, false])
            .max_height(hauteur_tableau)
            .show(ui, |ui| {
                egui::Grid::new("grille_cordes")
                    .num_columns(7)
                    .striped(true)
                    .spacing([14.0, 4.0])
                    .min_col_width(64.0)
                    .show(ui, |ui| {
                        ui.strong("N°");
                        ui.strong("Note");
                        ui.strong("Fréquence");
                        ui.strong("Forme d'onde");
                        ui.strong("Volume");
                        ui.strong("Couleur");
                        ui.strong("Active");
                        ui.end_row();

                        for (i, corde) in self.config.cordes.iter_mut().enumerate() {
                            let selectionnee = self.corde_selectionnee == i;
                            if ui
                                .selectable_label(selectionnee, format!("{}", i + 1))
                                .clicked()
                            {
                                self.corde_selectionnee = i;
                            }
                            if ui
                                .selectable_label(selectionnee, &corde.note)
                                .clicked()
                            {
                                self.corde_selectionnee = i;
                            }
                            ui.label(format!("{:.1} Hz", corde.frequence));
                            if ui
                                .selectable_label(selectionnee, corde.reglage.forme_onde.label())
                                .clicked()
                            {
                                self.corde_selectionnee = i;
                            }
                            let mut volume = corde.reglage.volume;
                            if ui
                                .add_sized(
                                    [ui.available_width(), 20.0],
                                    egui::Slider::new(&mut volume, 0.0..=1.0)
                                        .show_value(false)
                                        .fixed_decimals(2),
                                )
                                .changed()
                            {
                                changement_volume = Some((i, volume));
                            }
                            let couleur_corde = corde.couleur.unwrap_or_else(|| couleur_defaut(i));
                            let couleur32 = egui::Color32::from_rgb(
                                couleur_corde[0],
                                couleur_corde[1],
                                couleur_corde[2],
                            );
                            if egui::color_picker::show_color(
                                ui,
                                couleur32,
                                egui::vec2(24.0, 16.0),
                            )
                            .on_hover_text(format!(
                                "#{:02X}{:02X}{:02X}",
                                couleur_corde[0], couleur_corde[1], couleur_corde[2]
                            ))
                            .clicked()
                            {
                                self.corde_selectionnee = i;
                            }
                            if ui
                                .selectable_label(selectionnee, if corde.active { "●" } else { "○" })
                                .on_hover_text("Cliquez pour sélectionner la corde")
                                .clicked()
                            {
                                self.corde_selectionnee = i;
                            }
                            ui.end_row();
                        }
                    });
            });
        if let Some((index, volume)) = changement_volume {
            if self.volumes_lies {
                // Applique le même changement à toutes les cordes.
                let ancien = self.config.cordes[index].reglage.volume;
                let delta = volume - ancien;
                for c in &mut self.config.cordes {
                    c.reglage.volume = (c.reglage.volume + delta).clamp(0.0, 1.0);
                }
            } else {
                self.config.cordes[index].reglage.volume = volume;
            }
            self.modifie = true;
        }
    }

    /// Panneau latéral de réglage détaillé de la corde sélectionnée
    /// (fréquence, forme d'onde, enveloppe, aperçu de l'onde).
    fn panneau_reglages(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("panneau_reglages")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let index = self.corde_selectionnee;
                let corde = match self.config.cordes.get_mut(index) {
                    Some(c) => c,
                    None => return,
                };

                ui.heading(egui::RichText::new("Réglages du son").color(self.accent));
                ui.label(
                    egui::RichText::new(format!("Corde n°{} : {}", index + 1, corde.note))
                        .strong()
                        .size(15.0),
                );
                ui.separator();

                let mut frequence = corde.frequence;
                if ui
                    .add(
                        egui::Slider::new(&mut frequence, 30.0..=2200.0)
                            .logarithmic(true)
                            .text("Fréquence (Hz)"),
                    )
                    .changed()
                {
                    corde.frequence = frequence;
                    corde.rafraichir_note();
                    self.modifie = true;
                }
                ui.small(format!(
                    "Note détectée : {}  ({:.2} Hz)",
                    crate::model::nom_note(corde.frequence),
                    corde.frequence
                ));
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label("Forme d'onde :");
                    egui::ComboBox::from_id_salt("forme_onde")
                        .selected_text(corde.reglage.forme_onde.label())
                        .show_ui(ui, |ui| {
                            for forme in FormeOnde::toutes() {
                                ui.selectable_value(
                                    &mut corde.reglage.forme_onde,
                                    forme,
                                    forme.label(),
                                );
                            }
                        });
                });
                if ui
                    .checkbox(&mut corde.active, "Corde active")
                    .changed()
                {
                    self.modifie = true;
                }

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Enveloppe").strong());
                if ui
                    .add(
                        egui::Slider::new(&mut corde.reglage.volume, 0.0..=1.0)
                            .text("Volume"),
                    )
                    .changed()
                {
                    self.modifie = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut corde.reglage.attaque_ms, 0.0..=200.0)
                            .text("Attaque (ms)"),
                    )
                    .changed()
                {
                    self.modifie = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut corde.reglage.relache_ms, 0.0..=1000.0)
                            .text("Relâche (ms)"),
                    )
                    .changed()
                {
                    self.modifie = true;
                }

                ui.add_space(10.0);
                if ui
                    .button(egui::RichText::new("▶ Prévisualiser").color(self.accent))
                    .clicked()
                {
                    if let Some(audio) = &self.audio {
                        audio.jouer(corde.frequence, &corde.reglage);
                    }
                }
                if ui.button("Réinitialiser cette corde").clicked() {
                    *corde = Corde::nouvelle(index);
                    self.modifie = true;
                }

                ui.add_space(12.0);
                ui.label(egui::RichText::new("Aperçu de l'onde").strong());
                let hauteur_preview = 90.0;
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), hauteur_preview),
                    egui::Sense::hover(),
                );
                let couleur_corde = corde.couleur.unwrap_or_else(|| couleur_defaut(index));
                let couleur_corde_rgb =
                    egui::Color32::from_rgb(couleur_corde[0], couleur_corde[1], couleur_corde[2]);
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, egui::CornerRadius::same(6), FOND_APERCU);
                let points: Vec<egui::Pos2> = (0..200)
                    .map(|i| {
                        let ratio = i as f32 / 199.0;
                        let t = ratio * (2.0 / corde.frequence.max(1.0));
                        let echantillon =
                            corde.reglage.forme_onde.echantillon(t, corde.frequence);
                        let y = rect.center().y
                            - echantillon * corde.reglage.volume * (hauteur_preview * 0.4);
                        egui::pos2(rect.min.x + ratio * rect.width(), y)
                    })
                    .collect();
                painter.add(egui::Shape::line(
                    points,
                    egui::Stroke::new(2.0, couleur_corde_rgb),
                ));

                ui.add_space(12.0);
                ui.label(egui::RichText::new("Couleur de la note").strong());
                let couleur = corde.couleur.unwrap_or_else(|| couleur_defaut(index));
                let mut couleur32 = egui::Color32::from_rgb(couleur[0], couleur[1], couleur[2]);
                let largeur_picker = if self.picker_agrandi { 260.0 } else { 240.0 };
                ui.spacing_mut().slider_width = largeur_picker;
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(largeur_picker, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            if egui::color_picker::color_picker_color32(
                                ui,
                                &mut couleur32,
                                egui::color_picker::Alpha::Opaque,
                            ) {
                                corde.couleur = Some([couleur32.r(), couleur32.g(), couleur32.b()]);
                                self.modifie = true;
                            }
                            let hex = format!(
                                "#{:02X}{:02X}{:02X}",
                                couleur32.r(),
                                couleur32.g(),
                                couleur32.b()
                            );
                            ui.add_sized(
                                egui::vec2(largeur_picker, 20.0),
                                egui::Label::new(egui::RichText::new(hex).monospace())
                                    .halign(egui::Align::Center),
                            );
                        },
                    );
                });
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(if self.picker_agrandi { "⤡ Réduire" } else { "⤢ Agrandir" })
                            .on_hover_text("Agrandit ou réduit le sélecteur de couleur")
                            .clicked()
                        {
                            self.picker_agrandi = !self.picker_agrandi;
                        }
                        if ui
                            .button("💧 Prélever")
                            .on_hover_text("Choisit une couleur à l'écran")
                            .clicked()
                        {
                            crate::pipette::ouvrir(ui.ctx());
                        }
                    });
                });

                ui.add_space(12.0);
                ui.separator();
                ui.label(egui::RichText::new("Liaison série").strong());
                ui.add_enabled_ui(false, |ui| {
                    egui::ComboBox::from_id_salt("port_serie")
                        .selected_text("Aucun port")
                        .show_ui(ui, |_| {});
                });
                ui.small(
                    "Connexion à l'Arduino pour appliquer les réglages et jouer les notes.",
                );
            });
    }

    /// Barre de statut : fichier courant, état de l'audio et messages.
    fn barre_statut(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let nom_fichier = self
                .fichier
                .as_ref()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "non enregistré".to_string());
            ui.label(
                egui::RichText::new(nom_fichier)
                    .weak()
                    .monospace(),
            );
            if self.modifie {
                ui.label(egui::RichText::new("● modifié").color(self.accent));
            }
            ui.separator();
            if self.audio.is_some() {
                ui.small("Son : actif");
            }
            if let Some(erreur) = &self.erreur_audio {
                ui.small(format!("Aucune sortie audio : {erreur}"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(message) = &self.message {
                    ui.label(egui::RichText::new(message).weak());
                }
            });
        });
    }
}

impl eframe::App for HarpeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.traiter_actions_menu(ui.ctx());
        if let Some(couleur) = crate::pipette::couleur_piochee() {
            if let Some(corde) = self.config.cordes.get_mut(self.corde_selectionnee) {
                corde.couleur = Some(couleur);
                self.modifie = true;
            }
        }

        egui::Panel::top("barre_outils").show(ui, |ui| {
            ui.add_space(6.0);
            self.barre_outils(ui);
            ui.add_space(6.0);
        });

        egui::Panel::bottom("barre_statut").show(ui, |ui| {
            ui.add_space(4.0);
            self.barre_statut(ui);
            ui.add_space(4.0);
        });

        let largeur_max = ui.ctx().content_rect().width() / 3.0;
        let panneau = if self.panneau_gauche {
            egui::Panel::left("panneau_reglages")
        } else {
            egui::Panel::right("panneau_reglages")
        };
        panneau
            .resizable(true)
            .default_size(320.0)
            .min_size(260.0)
            .max_size(largeur_max)
            .show(ui, |ui| {
                ui.add_space(6.0);
                self.panneau_reglages(ui);
                ui.add_space(6.0);
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(6.0);
            self.zone_harpe(ui);
            self.tableau_cordes(ui);
        });
    }
}
