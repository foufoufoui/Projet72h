use eframe::egui;

/// Les thèmes visuels proposés par l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Sombre,
    Clair,
    Contraste,
}

impl Theme {
    /// Tous les thèmes, dans l'ordre d'affichage du sélecteur et du menu.
    pub const TOUTES: [Theme; 3] = [Theme::Sombre, Theme::Clair, Theme::Contraste];

    /// Nom affiché du thème.
    pub fn label(&self) -> &'static str {
        match self {
            Theme::Sombre => "Sombre",
            Theme::Clair => "Clair",
            Theme::Contraste => "Contraste",
        }
    }
}

/// Couleur d'accent Claude (terracotta/clay) adaptée au thème.
pub fn accent(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Sombre => egui::Color32::from_rgb(217, 119, 87),   // #D97757 Clay
        Theme::Clair => egui::Color32::from_rgb(180, 95, 65),     // Plus sombre sur fond clair
        Theme::Contraste => egui::Color32::from_rgb(230, 135, 100), // Plus vif pour contraste
    }
}

/// Couleur de fond de l'aperçu d'onde, adaptée au thème.
pub fn fond_apercu(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Sombre => egui::Color32::from_rgb(25, 25, 23),      // #191917 (proche de #141413)
        Theme::Clair => egui::Color32::from_rgb(240, 238, 230),    // #F0EEE6 Ivory Medium
        Theme::Contraste => egui::Color32::from_rgb(20, 20, 19),   // #141413 Slate Dark
    }
}

/// Couleur de grille pour l'aperçu d'onde (semi-transparente).
pub fn grille_apercu(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Sombre | Theme::Contraste => egui::Color32::from_rgba_unmultiplied(250, 249, 245, 25), // Ivory Light
        Theme::Clair => egui::Color32::from_rgba_unmultiplied(20, 20, 19, 20),                       // Slate Dark
    }
}

/// Construit les couleurs de fond et de texte du thème et les applique au
/// contexte egui.
pub fn appliquer(theme: Theme, ctx: &egui::Context) {
    let visuels = match theme {
        Theme::Sombre => {
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::from_rgb(30, 30, 28);     // #1E1E1C (proche de #141413)
            v.window_fill = egui::Color32::from_rgb(20, 20, 19);    // #141413 Slate Dark
            v.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(176, 174, 165); // #B0AEA5 Cloud Medium
            v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(94, 93, 89);    // #5E5D59 Slate Light
            v
        }
        Theme::Clair => {
            let mut v = egui::Visuals::light();
            v.panel_fill = egui::Color32::from_rgb(250, 249, 245);  // #FAF9F5 Ivory Light
            v.window_fill = egui::Color32::from_rgb(240, 238, 230);  // #F0EEE6 Ivory Medium
            v.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(61, 61, 58);     // #3D3D3A Slate Medium
            v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(232, 230, 220);  // #E8E6DC Ivory Dark
            v
        }
        Theme::Contraste => {
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::from_rgb(20, 20, 19);     // #141413 Slate Dark
            v.window_fill = egui::Color32::from_rgb(30, 30, 28);    // #1E1E1C
            v.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(250, 249, 245); // #FAF9F5 Ivory Light
            v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(94, 93, 89);    // #5E5D59 Slate Light
            v
        }
    };
    ctx.set_visuals(visuels);
}
