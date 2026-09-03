use std::sync::Mutex;

use eframe::egui;

/// Couleur prélevée à l'écran, en attente d'application par l'application.
/// La pipette native appelle son gestionnaire sur le thread principal, puis
/// l'application la consomme à l'image suivante.
static COULEUR_PIOCHES: Mutex<Option<[u8; 3]>> = Mutex::new(None);

/// Renvoie la couleur prélevée à l'écran, s'il y en a une.
pub fn couleur_piochee() -> Option<[u8; 3]> {
    let mut garde = COULEUR_PIOCHES.lock().unwrap_or_else(|e| e.into_inner());
    garde.take()
}

/// Ouvre la pipette pour choisir une couleur à l'écran.
/// Sans effet sur les autres plateformes.
pub fn ouvrir(ctx: &egui::Context) {
    #[cfg(target_os = "macos")]
    mac::ouvrir(ctx);
}

#[cfg(target_os = "macos")]
mod mac {
    use eframe::egui;
    use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};
    use objc2_core_foundation::CGFloat;

    use super::COULEUR_PIOCHES;

    /// Lance la session de prélèvement : l'utilisateur clique sur l'écran et
    /// la couleur choisie est stockée puis appliquée à l'image suivante.
    pub fn ouvrir(ctx: &egui::Context) {
        let ctx = ctx.clone();
        let sampler = NSColorSampler::new();
        let block = block2::RcBlock::new(move |couleur: *mut NSColor| {
            if couleur.is_null() {
                return;
            }
            let couleur = unsafe { &*couleur };
            let srgb = couleur.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace());
            if let Some(srgb) = srgb {
                let mut r: CGFloat = 0.0;
                let mut g: CGFloat = 0.0;
                let mut b: CGFloat = 0.0;
                let mut a: CGFloat = 0.0;
                unsafe { srgb.getRed_green_blue_alpha(&mut r, &mut g, &mut b, &mut a) };
                let rgb = [
                    (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                ];
                *COULEUR_PIOCHES.lock().unwrap() = Some(rgb);
            }
            ctx.request_repaint();
        });
        // SAFETY : la méthode est appelée depuis le thread principal de
        // l'application, et le gestionnaire est convoqué sur ce même thread.
        unsafe { sampler.showSamplerWithSelectionHandler(&block) };
    }
}
