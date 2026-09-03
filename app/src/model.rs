use serde::{Deserialize, Serialize};

/// Nombre de cordes de la harpe.
pub const NOMBRE_CORDES: usize = 22;

/// Fréquence de référence : Do3 (C3).
pub const FREQUENCE_DO3: f32 = 130.81;

/// Noms des notes en français (index 0 = Do).
pub const NOMS_NOTES: [&str; 12] = [
    "Do", "Do#", "Ré", "Ré#", "Mi", "Fa", "Fa#", "Sol", "Sol#", "La", "La#", "Si",
];

/// Les formes d'onde disponibles pour le timbre d'une note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FormeOnde {
    #[default]
    Sinus,
    Carre,
    DentDeScie,
    Triangle,
}

impl FormeOnde {
    /// Nom français affiché dans l'interface.
    pub fn label(&self) -> &'static str {
        match self {
            FormeOnde::Sinus => "Sinus",
            FormeOnde::Carre => "Carré",
            FormeOnde::DentDeScie => "Dent de scie",
            FormeOnde::Triangle => "Triangle",
        }
    }

    /// Toutes les formes d'onde, dans l'ordre d'affichage.
    pub fn toutes() -> [FormeOnde; 4] {
        [
            FormeOnde::Sinus,
            FormeOnde::Carre,
            FormeOnde::DentDeScie,
            FormeOnde::Triangle,
        ]
    }

    /// Génère un échantillon normalisé entre -1 et 1 pour une fréquence donnée.
    pub fn echantillon(&self, t: f32, frequence: f32) -> f32 {
        let phase = (t * frequence).fract();
        match self {
            FormeOnde::Sinus => (2.0 * std::f32::consts::PI * phase).sin(),
            FormeOnde::Carre => {
                if phase < 0.5 {
                    0.7
                } else {
                    -0.7
                }
            }
            FormeOnde::DentDeScie => 2.0 * phase - 1.0,
            FormeOnde::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
        }
    }
}

/// Réglages sonores d'une corde : timbre, volume et enveloppe.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReglageSon {
    pub forme_onde: FormeOnde,
    pub volume: f32,
    pub attaque_ms: f32,
    pub relache_ms: f32,
}

impl Default for ReglageSon {
    /// Réglages de départ : sinus, volume modéré, enveloppe courte.
    fn default() -> Self {
        ReglageSon {
            forme_onde: FormeOnde::Sinus,
            volume: 0.8,
            attaque_ms: 8.0,
            relache_ms: 180.0,
        }
    }
}

/// Une corde de la harpe : note, fréquence, couleur et réglage sonore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Corde {
    pub note: String,
    pub frequence: f32,
    pub reglage: ReglageSon,
    pub active: bool,
    /// Couleur personnalisée RGB ; `None` = couleur automatique selon l'index.
    #[serde(default)]
    pub couleur: Option<[u8; 3]>,
}

impl Corde {
    /// Crée une corde avec la note chromatique par défaut (Do3 + index).
    pub fn nouvelle(index: usize) -> Self {
        let semitones = index as f32;
        Corde {
            note: nom_note(FREQUENCE_DO3 * 2.0f32.powf(semitones / 12.0)),
            frequence: FREQUENCE_DO3 * 2.0f32.powf(semitones / 12.0),
            reglage: ReglageSon::default(),
            active: true,
            couleur: None,
        }
    }

    /// Met à jour le nom de note affiché à partir de la fréquence courante.
    pub fn rafraichir_note(&mut self) {
        self.note = nom_note(self.frequence);
    }
}

/// La configuration complète de la harpe (toutes les cordes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarpeConfig {
    pub cordes: Vec<Corde>,
}

impl Default for HarpeConfig {
    /// Construit la harpe par défaut : 22 cordes chromatiques, de Do3 à Ré5.
    fn default() -> Self {
        HarpeConfig {
            cordes: (0..NOMBRE_CORDES).map(Corde::nouvelle).collect(),
        }
    }
}

/// Nom de la note la plus proche d'une fréquence (ex. "La4", "Fa#3").
pub fn nom_note(frequence: f32) -> String {
    if frequence <= 0.0 {
        return "?".to_string();
    }
    let do4 = FREQUENCE_DO3 * 4.0; // 261.63 Hz
    let demi_tons = (12.0 * (frequence / do4).log2()).round() as i32;
    let octave = 4 + demi_tons.div_euclid(12);
    let index = demi_tons.rem_euclid(12) as usize;
    format!("{}{}", NOMS_NOTES[index], octave)
}

/// Couleur automatique d'une corde, répartie autour du cercle chromatique en
/// fonction de son index (sert de défaut quand aucune couleur n'est choisie).
pub fn couleur_defaut(index: usize) -> [u8; 3] {
    let teinte = index as f32 / NOMBRE_CORDES as f32;
    let (s, v) = (0.6, 0.55);
    let h = teinte * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}
