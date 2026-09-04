use std::path::PathBuf;

use eframe::egui;

use crate::audio::Audio;
use crate::config;
use crate::menus;
use crate::model::{couleur_defaut, Corde, FormeOnde, HarpeConfig};
use crate::theme::{self, Theme};

const ECART_PETIT: f32 = 6.0;
const ECART_MOYEN: f32 = 12.0;
const ECART_GRAND: f32 = 20.0;

/// Trace une ligne séparatrice pleine largeur à la position courante du
/// curseur, puis laisse respirer le contenu suivant.
fn ligne_separatrice(ui: &egui::Ui, stroke: egui::Stroke) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y;
    ui.painter().line_segment(
        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
        stroke,
    );
}

/// Petite palette de couleurs préréglées, proposée sur les plateformes sans
/// pipette d'écran native (Windows, Linux) en remplacement du prélèvement.
#[cfg(not(target_os = "macos"))]
const PALETTE: [[u8; 3]; 6] = [
    [217, 119, 87],  // Terracotta
    [224, 164, 88],  // Ambre
    [91, 141, 184],  // Ciel
    [127, 168, 122], // Sauge
    [154, 143, 191], // Lavande
    [199, 123, 156], // Rose
];

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
            // Marque en une ligne : pastille + nom + sous-titre
            let (rect_marque, _) =
                ui.allocate_exact_size(egui::vec2(12.0, 16.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect_marque.center(), 4.5, self.accent);
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Harpe sans corde").size(14.0).strong());
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Réglage des cordes").size(11.0).weak());
            ui.add_space(ECART_GRAND);

            // Action primaire
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Nouveau").size(12.0),
                    )
                    .corner_radius(20.0),
                )
                .clicked()
            {
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

            ui.add_space(ECART_MOYEN);

            // Action musicale (accent)
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("▶ Jouer un harpège")
                            .size(12.0)
                            .color(self.accent),
                    )
                    .corner_radius(20.0)
                    .stroke(egui::Stroke::new(1.0, self.accent)),
                )
                .on_hover_text("Joue toutes les cordes")
                .clicked()
            {
                if let Some(audio) = &self.audio {
                    audio.jouer_arpege(&self.config.cordes);
                }
            }

            // Groupe droit (position panneau + thème)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(
                                if self.panneau_gauche {
                                    "Panneau à gauche"
                                } else {
                                    "Panneau à droite"
                                },
                            )
                            .size(12.0),
                        )
                        .corner_radius(20.0),
                    )
                    .on_hover_text("Déplace le panneau de réglages à gauche ou à droite")
                    .clicked()
                {
                    self.basculer_panneau();
                }
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
                ui.add_space(8.0);
            });
        });

        // Séparateur sous la barre : fine ligne pleine largeur
        ligne_separatrice(
            ui,
            egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color.gamma_multiply(0.5),
            ),
        );
    }

    /// Zone principale : les 22 cordes cliquables (couleur, note, fréquence).
    fn zone_harpe(&mut self, ui: &mut egui::Ui) {
        ui.add_space(ECART_MOYEN);
        egui::Frame::new()
            .inner_margin(ECART_GRAND)
            .corner_radius(12.0)
            .show(ui, |ui| {
                // En-tête
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Les 22 cordes")
                            .size(18.0)
                            .strong()
                            .color(self.accent),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Cliquer pour écouter")
                                .size(11.0)
                                .weak(),
                        );
                    });
                });
                ui.add_space(ECART_MOYEN);

                // Scroll horizontal avec cartes de cordes
                let hauteur_carte = 120.0;
                let largeur_carte = 72.0;
                let espacement = ECART_MOYEN;

                egui::ScrollArea::horizontal()
                    .id_salt("zone_cordes")
                    .max_height(hauteur_carte + 24.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(ECART_PETIT);
                            for (i, corde) in self.config.cordes.iter().enumerate() {
                                let selectionnee = self.corde_selectionnee == i;
                                let couleur = egui::Color32::from_rgb(
                                    corde.couleur.unwrap_or_else(|| couleur_defaut(i))[0],
                                    corde.couleur.unwrap_or_else(|| couleur_defaut(i))[1],
                                    corde.couleur.unwrap_or_else(|| couleur_defaut(i))[2],
                                );
                                let couleur_affichee = if !corde.active {
                                    couleur.gamma_multiply(0.3)
                                } else {
                                    couleur
                                };

                                let (rect, reponse) = ui.allocate_exact_size(
                                    egui::vec2(largeur_carte, hauteur_carte),
                                    egui::Sense::click(),
                                );

                                let painter = ui.painter();

                                // Ombre subtile
                                if selectionnee {
                                    painter.rect_filled(
                                        rect.expand(3.0),
                                        egui::CornerRadius::same(10),
                                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 40),
                                    );
                                }

                                // Fond de la carte
                                let arrondi = egui::CornerRadius::same(8);
                                let fond_carte = if selectionnee {
                                    ui.visuals().widgets.active.bg_fill
                                } else {
                                    ui.visuals().widgets.noninteractive.bg_fill
                                };
                                painter.rect_filled(rect, arrondi, fond_carte);

                                // Bande de couleur en haut
                                let mut rect_bande = rect;
                                rect_bande.max.y = rect.min.y + 36.0;
                                painter.rect_filled(
                                    rect_bande,
                                    egui::CornerRadius {
                                        nw: 8,
                                        ne: 8,
                                        sw: 0,
                                        se: 0,
                                    },
                                    couleur_affichee,
                                );

                                // Numéro sur la bande
                                painter.text(
                                    egui::pos2(rect.min.x + 8.0, rect.min.y + 12.0),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{}", i + 1),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                                );

                                // Note (centrée)
                                painter.text(
                                    egui::pos2(rect.center().x, rect.min.y + 56.0),
                                    egui::Align2::CENTER_CENTER,
                                    &corde.note,
                                    egui::FontId::proportional(16.0),
                                    if selectionnee { self.accent } else { ui.visuals().text_color() },
                                );

                                // Fréquence
                                painter.text(
                                    egui::pos2(rect.center().x, rect.min.y + 76.0),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{:.0} Hz", corde.frequence),
                                    egui::FontId::proportional(10.0),
                                    ui.visuals().weak_text_color(),
                                );

                                // Indicateur actif/inactif
                                let y_indicateur = rect.max.y - 14.0;
                                let couleur_indicateur = if corde.active {
                                    egui::Color32::from_rgb(120, 180, 120)
                                } else {
                                    egui::Color32::from_rgb(120, 120, 120)
                                };
                                painter.circle_filled(
                                    egui::pos2(rect.center().x, y_indicateur),
                                    3.0,
                                    couleur_indicateur,
                                );

                                // Bordure de sélection
                                if selectionnee {
                                    painter.rect_stroke(
                                        rect,
                                        arrondi,
                                        egui::Stroke::new(2.0, self.accent),
                                        egui::StrokeKind::Inside,
                                    );
                                }

                                // Effet de survol
                                if reponse.hovered() && !selectionnee {
                                    painter.rect_filled(
                                        rect,
                                        arrondi,
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                                    );
                                }

                                if reponse.clicked() {
                                    self.corde_selectionnee = i;
                                    self.jouer_corde(i);
                                }

                                ui.add_space(espacement);
                            }
                            ui.add_space(ECART_PETIT);
                        });
                    });
            });
    }

    /// Vue d'ensemble en tableau : sélection et réglage rapide de chaque corde.
    fn tableau_cordes(&mut self, ui: &mut egui::Ui) {
        ui.add_space(ECART_MOYEN);
        egui::Frame::new()
            .inner_margin(ECART_GRAND)
            .corner_radius(12.0)
            .show(ui, |ui| {
                // En-tête de section
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Vue d'ensemble")
                            .size(18.0)
                            .strong()
                            .color(self.accent),
                    );
                    let actif = self.volumes_lies;
                    let bouton = if actif {
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("Volumes liés").color(self.accent),
                            )
                            .corner_radius(20.0),
                        )
                    } else {
                        ui.add(
                            egui::Button::new("Volumes indépendants").corner_radius(20.0),
                        )
                    }
                    .on_hover_text(
                        "Régler le volume d'une corde applique le même changement à toutes les cordes en même temps.",
                    );
                    if bouton.clicked() {
                        self.volumes_lies = !actif;
                    }
                });
                ui.add_space(ECART_MOYEN);

                let hauteur_tableau = ui.available_height().max(200.0);
                let mut changement_volume: Option<(usize, f32)> = None;
                let mut basculer_active: Option<usize> = None;

                egui::ScrollArea::vertical()
                    .id_salt("tableau_cordes")
                    .auto_shrink([false, false])
                    .max_height(hauteur_tableau)
                    .show(ui, |ui| {
                        // Séparateur haut
                        let rect_sep = ui.available_rect_before_wrap();
                        ui.painter().line_segment(
                            [
                                egui::pos2(rect_sep.min.x, rect_sep.min.y),
                                egui::pos2(rect_sep.max.x, rect_sep.min.y),
                            ],
                            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                        );
                        ui.add_space(ECART_PETIT);

                        // Lignes du tableau
                        for (i, corde) in self.config.cordes.iter_mut().enumerate() {
                            let selectionnee = self.corde_selectionnee == i;
                            let hauteur_ligne: f32 = 36.0;

                            ui.horizontal(|ui| {
                                ui.add_space(ECART_MOYEN);

                                // N° (cliquable pour sélectionner)
                                let (rect_num, reponse_num) = ui.allocate_exact_size(
                                    egui::vec2(36.0, hauteur_ligne),
                                    egui::Sense::click(),
                                );
                                if selectionnee {
                                    let rect_selection = egui::Rect::from_center_size(
                                        rect_num.center(),
                                        egui::vec2(28.0, 24.0),
                                    );
                                    ui.painter().rect_filled(
                                        rect_selection,
                                        egui::CornerRadius::same(6),
                                        self.accent.gamma_multiply(0.15),
                                    );
                                }
                                ui.painter().text(
                                    rect_num.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{}", i + 1),
                                    egui::FontId::proportional(13.0),
                                    if selectionnee { self.accent } else { ui.visuals().text_color() },
                                );
                                if reponse_num.clicked() {
                                    self.corde_selectionnee = i;
                                }

                                // Note (cliquable pour sélectionner)
                                let (rect_note, reponse_note) = ui.allocate_exact_size(
                                    egui::vec2(72.0, hauteur_ligne),
                                    egui::Sense::click(),
                                );
                                ui.painter().text(
                                    rect_note.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &corde.note,
                                    egui::FontId::proportional(14.0),
                                    if selectionnee { self.accent } else { ui.visuals().text_color() },
                                );
                                if reponse_note.clicked() {
                                    self.corde_selectionnee = i;
                                }

                                // Fréquence
                                let (rect_freq, reponse_freq) = ui.allocate_exact_size(
                                    egui::vec2(80.0, hauteur_ligne),
                                    egui::Sense::click(),
                                );
                                ui.painter().text(
                                    rect_freq.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{:.1} Hz", corde.frequence),
                                    egui::FontId::monospace(12.0),
                                    ui.visuals().weak_text_color(),
                                );
                                if reponse_freq.clicked() {
                                    self.corde_selectionnee = i;
                                }

                                // Forme d'onde
                                let (rect_forme, reponse_forme) = ui.allocate_exact_size(
                                    egui::vec2(96.0, hauteur_ligne),
                                    egui::Sense::click(),
                                );
                                ui.painter().text(
                                    rect_forme.center(),
                                    egui::Align2::CENTER_CENTER,
                                    corde.reglage.forme_onde.label(),
                                    egui::FontId::proportional(12.0),
                                    ui.visuals().weak_text_color(),
                                );
                                if reponse_forme.clicked() {
                                    self.corde_selectionnee = i;
                                }

                                // Volume (slider)
                                ui.add_space(ECART_MOYEN);
                                let mut volume = corde.reglage.volume;
                                if ui
                                    .add_sized(
                                        [110.0, 20.0],
                                        egui::Slider::new(&mut volume, 0.0..=1.0)
                                            .show_value(false)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                {
                                    changement_volume = Some((i, volume));
                                }

                                // Couleur (pastille ronde)
                                ui.add_space(ECART_MOYEN);
                                let couleur_corde = corde.couleur.unwrap_or_else(|| couleur_defaut(i));
                                let couleur32 = egui::Color32::from_rgb(
                                    couleur_corde[0],
                                    couleur_corde[1],
                                    couleur_corde[2],
                                );
                                let (rect_couleur, reponse_couleur) = ui.allocate_exact_size(
                                    egui::vec2(36.0, hauteur_ligne),
                                    egui::Sense::click(),
                                );
                                ui.painter().circle_filled(
                                    rect_couleur.center(),
                                    8.0,
                                    couleur32,
                                );
                                if reponse_couleur.clicked() {
                                    self.corde_selectionnee = i;
                                }

                                // État (cliquable pour activer/désactiver)
                                let label_etat = if corde.active { "Actif" } else { "Inactif" };
                                let couleur_etat = if corde.active {
                                    egui::Color32::from_rgb(120, 180, 120)
                                } else {
                                    egui::Color32::from_rgb(140, 140, 140)
                                };
                                let (rect_etat, reponse_etat) = ui.allocate_exact_size(
                                    egui::vec2(64.0, hauteur_ligne),
                                    egui::Sense::click(),
                                );
                                let rect_pilule = egui::Rect::from_center_size(
                                    rect_etat.center(),
                                    egui::vec2(56.0, 22.0),
                                );
                                ui.painter().rect_filled(
                                    rect_pilule,
                                    egui::CornerRadius::same(10),
                                    couleur_etat.gamma_multiply(0.2),
                                );
                                ui.painter().text(
                                    rect_etat.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label_etat,
                                    egui::FontId::proportional(11.0),
                                    couleur_etat,
                                );
                                if reponse_etat.clicked() {
                                    basculer_active = Some(i);
                                }
                            });

                            // Séparateur léger
                            let rect_sep_ligne = ui.available_rect_before_wrap();
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect_sep_ligne.min.x + 16.0, rect_sep_ligne.min.y),
                                    egui::pos2(rect_sep_ligne.max.x - 16.0, rect_sep_ligne.min.y),
                                ],
                                egui::Stroke::new(
                                    0.5,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color.gamma_multiply(0.5),
                                ),
                            );
                        }
                    });

                // Appliquer les changements après la boucle
                if let Some(index) = basculer_active {
                    self.config.cordes[index].active = !self.config.cordes[index].active;
                    self.modifie = true;
                }

                if let Some((index, volume)) = changement_volume {
                    if self.volumes_lies {
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
            });
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

                // En-tête : eyebrow + titre + sous-titre
                ui.label(
                    egui::RichText::new("RÉGLAGES")
                        .size(9.0)
                        .strong()
                        .monospace()
                        .color(self.accent),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("Corde n°{}", index + 1))
                        .size(24.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{} · {:.2} Hz", corde.note, corde.frequence))
                        .size(12.0)
                        .weak(),
                );
                ui.add_space(ECART_GRAND);

                let hairline = ui
                    .visuals()
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .color
                    .gamma_multiply(0.55);
                let marge = 14.0;

                // Carte unifiée : le son de la corde
                egui::Frame::new()
                    .inner_margin(marge)
                    .corner_radius(14.0)
                    .stroke(egui::Stroke::new(1.0, hairline))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Fréquence").strong().size(12.0));
                        ui.add_space(6.0);
                        let mut frequence = corde.frequence;
                        if ui
                            .add(
                                egui::Slider::new(&mut frequence, 30.0..=2200.0)
                                    .logarithmic(true)
                                    .text("Hz"),
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

                        ui.add_space(ECART_MOYEN);
                        ligne_separatrice(ui, egui::Stroke::new(1.0, hairline));
                        ui.add_space(ECART_MOYEN);

                        ui.label(egui::RichText::new("Forme d'onde").strong().size(12.0));
                        ui.add_space(6.0);
                        egui::ComboBox::from_id_salt("forme_onde")
                            .selected_text(corde.reglage.forme_onde.label())
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for forme in FormeOnde::toutes() {
                                    ui.selectable_value(
                                        &mut corde.reglage.forme_onde,
                                        forme,
                                        forme.label(),
                                    );
                                }
                            });
                        ui.add_space(4.0);
                        if ui
                            .checkbox(&mut corde.active, "Corde active")
                            .changed()
                        {
                            self.modifie = true;
                        }

                        ui.add_space(ECART_MOYEN);
                        ligne_separatrice(ui, egui::Stroke::new(1.0, hairline));
                        ui.add_space(ECART_MOYEN);

                        ui.label(egui::RichText::new("Enveloppe").strong().size(12.0));
                        ui.add_space(6.0);
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
                    });

                ui.add_space(ECART_GRAND);

                // Actions : deux pilules pleine largeur
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("▶ Prévisualiser")
                                .size(12.0)
                                .strong()
                                .color(self.accent),
                        )
                        .corner_radius(20.0)
                        .stroke(egui::Stroke::new(1.0, self.accent))
                        .min_size(egui::vec2(ui.available_width(), 32.0)),
                    )
                    .clicked()
                {
                    if let Some(audio) = &self.audio {
                        audio.jouer(corde.frequence, &corde.reglage);
                    }
                }
                ui.add_space(ECART_PETIT);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Réinitialiser").size(12.0),
                        )
                        .corner_radius(20.0)
                        .min_size(egui::vec2(ui.available_width(), 32.0)),
                    )
                    .on_hover_text("Réinitialiser cette corde")
                    .clicked()
                {
                    *corde = Corde::nouvelle(index);
                    self.modifie = true;
                }

                ui.add_space(ECART_GRAND);

                // Carte visuelle : onde + couleur
                egui::Frame::new()
                    .inner_margin(marge)
                    .corner_radius(14.0)
                    .stroke(egui::Stroke::new(1.0, hairline))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Aperçu de l'onde").strong().size(12.0));
                        ui.add_space(6.0);
                        let hauteur_preview = 80.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), hauteur_preview),
                            egui::Sense::hover(),
                        );
                        let couleur_corde = corde.couleur.unwrap_or_else(|| couleur_defaut(index));
                        let couleur_corde_rgb =
                            egui::Color32::from_rgb(couleur_corde[0], couleur_corde[1], couleur_corde[2]);
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, egui::CornerRadius::same(6), theme::fond_apercu(self.theme));

                        let couleur_grille = theme::grille_apercu(self.theme);
                        let stroke_grille = egui::Stroke::new(1.0, couleur_grille);
                        for i in 0..5 {
                            let y = rect.min.y + rect.height() * (i as f32 + 1.0) / 6.0;
                            painter.line_segment(
                                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                                stroke_grille,
                            );
                        }

                        let y_centre = rect.center().y;
                        painter.line_segment(
                            [
                                egui::pos2(rect.min.x, y_centre),
                                egui::pos2(rect.max.x, y_centre),
                            ],
                            egui::Stroke::new(1.0, couleur_grille.gamma_multiply(1.5)),
                        );

                        painter.text(
                            egui::pos2(rect.min.x + 4.0, rect.min.y + 2.0),
                            egui::Align2::LEFT_TOP,
                            "+1",
                            egui::FontId::proportional(9.0),
                            couleur_grille,
                        );
                        painter.text(
                            egui::pos2(rect.min.x + 4.0, rect.max.y - 12.0),
                            egui::Align2::LEFT_TOP,
                            "−1",
                            egui::FontId::proportional(9.0),
                            couleur_grille,
                        );

                        let points: Vec<egui::Pos2> = {
                            let harmoniques = corde.reglage.forme_onde.harmoniques();
                            let somme: f32 = harmoniques.iter().sum::<f32>().max(1e-3);
                            let partiels: Vec<(f32, f32)> = harmoniques
                                .iter()
                                .enumerate()
                                .filter(|(r, _)| corde.frequence * (r + 1) as f32 <= 16_000.0)
                                .map(|(r, poids)| {
                                    ((r + 1) as f32, poids / somme)
                                })
                                .collect();
                            (0..200)
                                .map(|i| {
                                    let ratio = i as f32 / 199.0;
                                    let t = ratio * (2.0 / corde.frequence.max(1.0));
                                    let echantillon: f32 = partiels
                                        .iter()
                                        .map(|(rang, poids)| {
                                            poids
                                                * (std::f32::consts::TAU
                                                    * corde.frequence
                                                    * rang
                                                    * t)
                                                .sin()
                                        })
                                        .sum();
                                    let y = rect.center().y
                                        - echantillon
                                            * corde.reglage.volume
                                            * (hauteur_preview * 0.4);
                                    egui::pos2(rect.min.x + ratio * rect.width(), y)
                                })
                                .collect()
                        };
                        painter.add(egui::Shape::line(
                            points,
                            egui::Stroke::new(2.0, couleur_corde_rgb),
                        ));

                        ui.add_space(ECART_MOYEN);
                        ligne_separatrice(ui, egui::Stroke::new(1.0, hairline));
                        ui.add_space(ECART_MOYEN);

                        ui.label(egui::RichText::new("Couleur de la note").strong().size(12.0));
                        ui.add_space(6.0);
                        let couleur = corde.couleur.unwrap_or_else(|| couleur_defaut(index));
                        let mut couleur32 = egui::Color32::from_rgb(couleur[0], couleur[1], couleur[2]);
                        let largeur_picker = ui.available_width().min(260.0);
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
                        ui.add_space(6.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(
                                                if self.picker_agrandi { "⤡ Réduire" } else { "⤢ Agrandir" },
                                            )
                                            .size(11.0),
                                        )
                                        .corner_radius(20.0),
                                    )
                                    .on_hover_text("Agrandit ou réduit le sélecteur de couleur")
                                    .clicked()
                                {
                                    self.picker_agrandi = !self.picker_agrandi;
                                }
                                #[cfg(target_os = "macos")]
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("💧 Prélever").size(11.0),
                                        )
                                        .corner_radius(20.0),
                                    )
                                    .on_hover_text("Choisit une couleur à l'écran")
                                    .clicked()
                                {
                                    crate::pipette::ouvrir(ui.ctx());
                                }
                                #[cfg(not(target_os = "macos"))]
                                {
                                    ui.add_space(ECART_PETIT);
                                    for couleur in PALETTE {
                                        let (rect_swatch, reponse) = ui.allocate_exact_size(
                                            egui::vec2(16.0, 16.0),
                                            egui::Sense::click(),
                                        );
                                        let est_active = corde.couleur == Some(couleur);
                                        ui.painter().circle_filled(
                                            rect_swatch.center(),
                                            6.0,
                                            egui::Color32::from_rgb(
                                                couleur[0],
                                                couleur[1],
                                                couleur[2],
                                            ),
                                        );
                                        if est_active {
                                            ui.painter().circle_stroke(
                                                rect_swatch.center(),
                                                8.0,
                                                egui::Stroke::new(1.5, self.accent),
                                            );
                                        }
                                        if reponse.clicked() {
                                            corde.couleur = Some(couleur);
                                            self.modifie = true;
                                        }
                                        reponse.on_hover_cursor(egui::CursorIcon::PointingHand);
                                    }
                                }
                            });
                        });
                    });

                ui.add_space(ECART_GRAND);

                // Liaison série : pied discret, sans encadrement
                ui.label(
                    egui::RichText::new("LIAISON SÉRIE")
                        .size(9.0)
                        .strong()
                        .monospace()
                        .weak(),
                );
                ui.add_space(6.0);
                ui.add_enabled_ui(false, |ui| {
                    ui.horizontal(|ui| {
                        let (rect_point, _) =
                            ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(rect_point.center(), 3.0, hairline);
                        ui.label(egui::RichText::new("Aucun port connecté").size(12.0));
                    });
                });
                ui.add_space(4.0);
                ui.small("Connexion à l'Arduino pour appliquer les réglages et jouer les notes.");
            });
    }

    /// Barre de statut : fichier courant, état de l'audio et messages.
    fn barre_statut(&mut self, ui: &mut egui::Ui) {
        ligne_separatrice(
            ui,
            egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color.gamma_multiply(0.5),
            ),
        );
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(ECART_GRAND as i8, ECART_PETIT as i8))
            .corner_radius(0.0)
            .show(ui, |ui| {
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
            });
    }
}

impl eframe::App for HarpeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        menus::capter_raccourcis(ui.ctx());
        self.traiter_actions_menu(ui.ctx());
        if let Some(couleur) = crate::pipette::couleur_piochee() {
            if let Some(corde) = self.config.cordes.get_mut(self.corde_selectionnee) {
                corde.couleur = Some(couleur);
                self.modifie = true;
            }
        }

        egui::Panel::top("barre_outils")
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_space(4.0);
                self.barre_outils(ui);
                ui.add_space(2.0);
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
                ui.add_space(ECART_PETIT);
                self.panneau_reglages(ui);
                ui.add_space(ECART_PETIT);
            });

        egui::Panel::bottom("barre_statut")
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_space(ECART_PETIT);
                self.barre_statut(ui);
                ui.add_space(ECART_PETIT);
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(ECART_PETIT);
            self.zone_harpe(ui);
            self.tableau_cordes(ui);
        });
    }
}
