use std::num::NonZero;
use std::time::Duration;

use rodio::{mixer::Mixer, DeviceSinkBuilder, Source};

use crate::model::{Corde, ReglageSon};

const TAUX_ECHANTILLON: u32 = 44_100;
/// Durée du « corps » de la note (entre l'attaque et la relâche).
const DUREE_CORPS_S: f32 = 0.35;

/// Gère la sortie audio (aperçu des sons de la harpe).
pub struct Audio {
    _sortie: rodio::MixerDeviceSink,
    mixeur: Mixer,
}

impl Audio {
    /// Ouvre la sortie audio par défaut et en récupère le mixeur.
    pub fn new() -> Result<Self, String> {
        let sortie = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        let mixeur = sortie.mixer().clone();
        Ok(Audio {
            _sortie: sortie,
            mixeur,
        })
    }

    /// Joue une note avec la forme d'onde et l'enveloppe données.
    pub fn jouer(&self, frequence: f32, reglage: &ReglageSon) {
        self.mixeur.add(Note::nouvelle(frequence, *reglage));
    }

    /// Joue toutes les cordes actives en enchaînement (arpège).
    pub fn jouer_arpege(&self, cordes: &[Corde]) {
        let mixeur = self.mixeur.clone();
        let notes: Vec<(f32, ReglageSon)> = cordes
            .iter()
            .filter(|c| c.active)
            .map(|c| (c.frequence, c.reglage))
            .collect();
        std::thread::spawn(move || {
            for (frequence, reglage) in notes {
                mixeur.add(Note::nouvelle(frequence, reglage));
                std::thread::sleep(Duration::from_millis(140));
            }
        });
    }
}

/// Partiel sinusoïdal du synthétiseur additif. Une corde pincée est une
/// somme de partiels qui s'éteignent d'autant plus vite qu'ils sont aigus :
/// c'est ce contre-champ progressif qui donne la chaleur du pizzicato.
struct Partiel {
    frequence: f32,
    amplitude: f32,
    /// Constante de décroissance propre au partiel, en secondes.
    decroissance: f32,
    /// Déphasage à l'attaque, pour lisser le départ de la note.
    phase: f32,
}

/// Source générée en direct : synthèse additive (fondamental + partiels du
/// timbre choisi) avec attaque douce sans déclic et décroissance naturelle
/// de type corde pincée.
struct Note {
    partiels: Vec<Partiel>,
    volume: f32,
    attaque_s: f32,
    total_echantillons: u64,
    position: u64,
}

impl Note {
    /// Crée une note à partir du timbre, du volume et de l'enveloppe choisis.
    /// Le spectre est la série de Fourier de la forme d'onde, chaque partiel
    /// déclinant plus vite que le précédent (effet de « bloom »).
    fn nouvelle(frequence: f32, reglage: ReglageSon) -> Self {
        let attaque_s = (reglage.attaque_ms / 1000.0).clamp(0.001, 1.0);
        let relache_s = (reglage.relache_ms / 1000.0).max(0.01);
        let duree_s = (attaque_s + DUREE_CORPS_S + relache_s).max(0.05);
        // Constante de décroissance globale : la note s'éteint en douceur
        // pendant toute la durée visible (≈ −60 dB à la fin).
        let tau_s = (DUREE_CORPS_S + relache_s) / 7.0;

        let harmoniques = reglage.forme_onde.harmoniques();
        let somme_normalisation: f32 = harmoniques.iter().sum::<f32>().max(1e-3);
        let plafond_hz = 16_000.0;

        let mut partiels = Vec::with_capacity(harmoniques.len());
        for (index, poids) in harmoniques.iter().enumerate() {
            let rang_partiel = (index + 1) as f32;
            let frequence_partiel = frequence * rang_partiel;
            if frequence_partiel > plafond_hz {
                continue;
            }
            partiels.push(Partiel {
                frequence: frequence_partiel,
                amplitude: (poids / somme_normalisation) * 0.95,
                // Les partiels élevés s'éteignent nettement plus vite.
                decroissance: tau_s / rang_partiel.powf(1.3),
                phase: rang_partiel * 0.35,
            });
        }
        if partiels.is_empty() {
            partiels.push(Partiel {
                frequence,
                amplitude: 0.9,
                decroissance: tau_s,
                phase: 0.0,
            });
        }

        Note {
            partiels,
            volume: reglage.volume,
            attaque_s,
            total_echantillons: (duree_s * TAUX_ECHANTILLON as f32) as u64,
            position: 0,
        }
    }
}

impl Iterator for Note {
    type Item = f32;

    /// Produit l'échantillon suivant : somme des partiels décroissants, avec
    /// une attaque en arc de cercle (dérivée nulle au départ, donc pas de
    /// claquement au démarrage). Renvoie `None` une fois la note terminée.
    fn next(&mut self) -> Option<f32> {
        if self.position >= self.total_echantillons {
            return None;
        }
        let t = self.position as f32 / TAUX_ECHANTILLON as f32;
        let t_apres_attaque = (t - self.attaque_s).max(0.0);

        let enveloppe_attaque = if t < self.attaque_s {
            (1.0 - (std::f32::consts::PI * t / self.attaque_s).cos()) * 0.5
        } else {
            1.0
        };

        let mut echantillon = 0.0;
        for partiel in &self.partiels {
            let angle = std::f32::consts::TAU * partiel.frequence * t + partiel.phase;
            let fondu_partiel = (-t_apres_attaque / partiel.decroissance).exp();
            echantillon += partiel.amplitude * fondu_partiel * angle.sin();
        }

        self.position += 1;
        Some((echantillon * enveloppe_attaque * self.volume).clamp(-1.0, 1.0))
    }
}

impl Source for Note {
    /// Nombre d'échantillons fournis par lot (512 à la fois).
    fn current_span_len(&self) -> Option<usize> {
        Some(512)
    }

    /// La note est mono (un seul canal).
    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(1).unwrap()
    }

    /// Taux d'échantillonnage de génération (44,1 kHz).
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(TAUX_ECHANTILLON).unwrap()
    }

    /// Durée totale de la note (pour l'arrêt automatique du rendu).
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(
            (self.total_echantillons * 1000) / TAUX_ECHANTILLON as u64,
        ))
    }
}
