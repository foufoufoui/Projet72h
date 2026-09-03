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

/// Source générée en direct : forme d'onde + enveloppe attaque/relâche.
struct Note {
    frequence: f32,
    reglage: ReglageSon,
    total_echantillons: u64,
    position: u64,
}

impl Note {
    /// Crée une note avec une durée totale calculée à partir des réglages
    /// (attaque + corps + relâche), avec un minimum de 50 ms.
    fn nouvelle(frequence: f32, reglage: ReglageSon) -> Self {
        let duree_s = (reglage.attaque_ms / 1000.0 + DUREE_CORPS_S + reglage.relache_ms / 1000.0)
            .max(0.05);
        Note {
            frequence,
            reglage,
            total_echantillons: (duree_s * TAUX_ECHANTILLON as f32) as u64,
            position: 0,
        }
    }
}

impl Iterator for Note {
    type Item = f32;

    /// Produit l'échantillon suivant : forme d'onde × enveloppe × volume.
    /// Renvoie `None` une fois la note entièrement générée.
    fn next(&mut self) -> Option<f32> {
        if self.position >= self.total_echantillons {
            return None;
        }
        let t = self.position as f32 / TAUX_ECHANTILLON as f32;
        let attaque_s = (self.reglage.attaque_ms / 1000.0).max(0.001);
        let relache_s = (self.reglage.relache_ms / 1000.0).max(0.001);

        let enveloppe = if t < attaque_s {
            t / attaque_s
        } else if t < attaque_s + DUREE_CORPS_S {
            1.0
        } else {
            let dt = t - attaque_s - DUREE_CORPS_S;
            (1.0 - dt / relache_s).clamp(0.0, 1.0)
        };

        let echantillon =
            self.reglage.forme_onde.echantillon(t, self.frequence) * enveloppe * self.reglage.volume;
        self.position += 1;
        Some(echantillon)
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
