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

/// Couleur d'accent (dorée) adaptée au thème pour rester lisible.
pub fn accent(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Sombre => egui::Color32::from_rgb(212, 175, 55),
        Theme::Clair => egui::Color32::from_rgb(166, 124, 0),
        Theme::Contraste => egui::Color32::from_rgb(255, 200, 60),
    }
}

/// Construit les couleurs de fond et de texte du thème et les applique au
/// contexte egui.
pub fn appliquer(theme: Theme, ctx: &egui::Context) {
    let mut visuels = match theme {
        Theme::Sombre => {
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::from_rgb(31, 32, 32);
            v.window_fill = egui::Color32::from_rgb(10, 10, 12);
            v
        }
        Theme::Clair => {
            let mut v = egui::Visuals::light();
            v.panel_fill = egui::Color32::from_rgb(240, 240, 236);
            v.window_fill = egui::Color32::from_rgb(250, 250, 247);
            v
        }
        Theme::Contraste => {
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::from_rgb(10, 10, 12);
            v.window_fill = egui::Color32::from_rgb(18, 18, 20);
            v
        }
    };
    if theme != Theme::Clair {
        visuels.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(225, 220, 210);
    }
    ctx.set_visuals(visuels);
}
