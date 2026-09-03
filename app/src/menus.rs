use std::sync::{Mutex, OnceLock};

use eframe::egui;

use crate::theme::Theme;

/// Une action demandée par l'utilisateur via le menu natif macOS, mise en
/// file pour être traitée par `HarpeApp` à l'image suivante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMenu {
    Nouveau,
    Ouvrir,
    Enregistrer,
    EnregistrerSous,
    BasculerPanneau,
    Theme(Theme),
    Quitter,
}

/// File des actions de menu en attente de traitement par l'application.
/// Le menu natif et l'application egui peuvent tourner sur des threads
/// différents, d'où le `Mutex`.
static ACTIONS: Mutex<Vec<ActionMenu>> = Mutex::new(Vec::new());
/// Contexte egui mémorisé pour demander des reprises de peinture depuis le
/// gestionnaire de menu natif.
static CTX: OnceLock<egui::Context> = OnceLock::new();

/// Mémorise le contexte egui puis installe la barre de menus native sur macOS
/// (aucun effet sur les autres plateformes).
pub fn installer(ctx: &egui::Context) {
    let _ = CTX.set(ctx.clone());
    #[cfg(target_os = "macos")]
    mac::installer();
}

/// Vide la file et renvoie toutes les actions de menu en attente.
pub fn tirer_actions() -> Vec<ActionMenu> {
    let mut garde = ACTIONS.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *garde)
}

/// Met à jour le libellé de l'élément « Panneau » du menu natif (no-op hors
/// macOS).
pub fn mettre_a_jour_panneau(gauche: bool) {
    #[cfg(target_os = "macos")]
    mac::mettre_a_jour_panneau(gauche);
}

/// Synchronise la coche du thème actif dans le menu natif (no-op hors macOS).
pub fn mettre_a_jour_theme(theme: Theme) {
    #[cfg(target_os = "macos")]
    mac::mettre_a_jour_theme(theme);
}

#[cfg(target_os = "macos")]
mod mac {
    use std::sync::{MutexGuard, OnceLock};

    use eframe::egui;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem,
        NSEventModifierFlags,
    };
    use objc2_foundation::{NSInteger, NSObject, NSString};

    use super::{ActionMenu, ACTIONS, CTX};
    use crate::theme::Theme;

    /// `tag` des éléments de menu, utilisé pour identifier l'élément cliqué
    /// dans le gestionnaire d'action.
    const TAG_PANNEAU: NSInteger = 100;
    const TAG_THEME_SOMBRE: NSInteger = 101;
    const TAG_THEME_CLAIR: NSInteger = 102;
    const TAG_THEME_CONTRASTE: NSInteger = 103;

    const TAG_NOUVEAU: NSInteger = 200;
    const TAG_OUVRIR: NSInteger = 201;
    const TAG_ENREGISTRER: NSInteger = 202;
    const TAG_ENREGISTRER_SOUS: NSInteger = 203;
    const TAG_QUITTER: NSInteger = 204;

    const TAG_REDUIRE: NSInteger = 300;
    const TAG_AGRANDIR: NSInteger = 301;
    const TAG_PLEIN_ECRAN: NSInteger = 302;
    const TAG_FERMER_FENETRE: NSInteger = 303;

    /// Pointeur brut vers la cible de menu, qui survit à toute l'application
    /// (l'objet est « oublié » volontairement après installation).
    struct CiblePtr(*const MenuTarget);
    // La cible est créée et oubliée sur le thread principal, puis utilisée
    // uniquement depuis ce même thread.
    unsafe impl Send for CiblePtr {}
    unsafe impl Sync for CiblePtr {}

    static TARGET: OnceLock<CiblePtr> = OnceLock::new();

    /// Construit la barre de menus complète (application, Fichier, Affichage,
    /// Fenêtre), relie chaque élément à la cible et l'assigne à l'application.
    pub fn installer() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };

        let app = NSApplication::sharedApplication(mtm);
        let menubar = NSMenu::new(mtm);

        let mut items: Vec<Retained<NSMenuItem>> = Vec::new();

        let menu_application = nouveau_menu_menubar(mtm, &menubar, "Harpe sans corde");
        ajouter(
            mtm,
            &menu_application,
            &mut items,
            "Quitter Harpe sans corde",
            Some("q"),
            NSEventModifierFlags::Command,
            TAG_QUITTER,
        );

        let menu_fichier = nouveau_menu_menubar(mtm, &menubar, "Fichier");
        ajouter(
            mtm,
            &menu_fichier,
            &mut items,
            "Nouveau",
            Some("n"),
            NSEventModifierFlags::Command,
            TAG_NOUVEAU,
        );
        ajouter(
            mtm,
            &menu_fichier,
            &mut items,
            "Ouvrir…",
            Some("o"),
            NSEventModifierFlags::Command,
            TAG_OUVRIR,
        );
        ajouter(
            mtm,
            &menu_fichier,
            &mut items,
            "Enregistrer",
            Some("s"),
            NSEventModifierFlags::Command,
            TAG_ENREGISTRER,
        );
        ajouter(
            mtm,
            &menu_fichier,
            &mut items,
            "Enregistrer sous…",
            Some("s"),
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
            TAG_ENREGISTRER_SOUS,
        );

        let menu_affichage = nouveau_menu_menubar(mtm, &menubar, "Affichage");
        let item_panneau = ajouter_avec_case(
            mtm,
            &menu_affichage,
            &mut items,
            "Panneau à droite",
            None,
            TAG_PANNEAU,
        );
        menu_affichage.addItem(&NSMenuItem::separatorItem(mtm));
        let theme_sombre = ajouter_avec_case(
            mtm,
            &menu_affichage,
            &mut items,
            "Thème : Sombre",
            None,
            TAG_THEME_SOMBRE,
        );
        let theme_clair = ajouter_avec_case(
            mtm,
            &menu_affichage,
            &mut items,
            "Thème : Clair",
            None,
            TAG_THEME_CLAIR,
        );
        let theme_contraste = ajouter_avec_case(
            mtm,
            &menu_affichage,
            &mut items,
            "Thème : Contraste",
            None,
            TAG_THEME_CONTRASTE,
        );

        let menu_fenetre = nouveau_menu_menubar(mtm, &menubar, "Fenêtre");
        ajouter(
            mtm,
            &menu_fenetre,
            &mut items,
            "Réduire",
            Some("m"),
            NSEventModifierFlags::Command,
            TAG_REDUIRE,
        );
        ajouter(
            mtm,
            &menu_fenetre,
            &mut items,
            "Agrandir",
            None,
            NSEventModifierFlags::empty(),
            TAG_AGRANDIR,
        );
        ajouter(
            mtm,
            &menu_fenetre,
            &mut items,
            "Plein écran",
            Some("f"),
            NSEventModifierFlags::Command | NSEventModifierFlags::Control,
            TAG_PLEIN_ECRAN,
        );
        menu_fenetre.addItem(&NSMenuItem::separatorItem(mtm));
        ajouter(
            mtm,
            &menu_fenetre,
            &mut items,
            "Fermer la fenêtre",
            Some("w"),
            NSEventModifierFlags::Command,
            TAG_FERMER_FENETRE,
        );

        let target = creer_cible(mtm, item_panneau, [theme_sombre, theme_clair, theme_contraste]);
        for item in &items {
            unsafe {
                item.setTarget(Some(&*target));
                item.setAction(Some(sel!(handleMenuAction:)));
            }
        }

        // Le `target` est volontairement oublié : NSMenuItem.target est une
        // référence faible (assign), la cible doit donc survivre à l'application.
        let ptr: *const MenuTarget = &*target;
        std::mem::forget(target);
        let _ = TARGET.set(CiblePtr(ptr));

        app.setMainMenu(Some(&menubar));
    }

    /// Crée un titre de menu avec un sous-menu, puis l'attache à la barre de
    /// menus. Renvoie le sous-menu pour y ajouter des éléments.
    fn nouveau_menu_menubar(
        mtm: MainThreadMarker,
        menubar: &NSMenu,
        titre: &str,
    ) -> Retained<NSMenu> {
        let item = NSMenuItem::new(mtm);
        item.setTitle(&NSString::from_str(titre));
        let menu = NSMenu::new(mtm);
        item.setSubmenu(Some(&menu));
        menubar.addItem(&item);
        menu
    }

    /// Crée un élément de menu d'action (optionnellement avec raccourci
    /// clavier), lui donne un `tag`, et le stocke dans la liste commune.
    fn ajouter(
        mtm: MainThreadMarker,
        menu: &NSMenu,
        items: &mut Vec<Retained<NSMenuItem>>,
        titre: &str,
        touche: Option<&str>,
        modificateurs: NSEventModifierFlags,
        tag: NSInteger,
    ) {
        let item = NSMenuItem::new(mtm);
        item.setTitle(&NSString::from_str(titre));
        if let Some(touche) = touche {
            item.setKeyEquivalent(&NSString::from_str(touche));
            item.setKeyEquivalentModifierMask(modificateurs);
        }
        item.setTag(tag);
        menu.addItem(&item);
        items.push(item);
    }

    /// Crée un élément de menu à cocher, le stocke, et le renvoie pour pouvoir
    /// ensuite modifier sa coche (thèmes, panneau).
    fn ajouter_avec_case(
        mtm: MainThreadMarker,
        menu: &NSMenu,
        items: &mut Vec<Retained<NSMenuItem>>,
        titre: &str,
        touche: Option<&str>,
        tag: NSInteger,
    ) -> Retained<NSMenuItem> {
        let item = NSMenuItem::new(mtm);
        item.setTitle(&NSString::from_str(titre));
        if let Some(touche) = touche {
            item.setKeyEquivalent(&NSString::from_str(touche));
        }
        item.setTag(tag);
        menu.addItem(&item);
        items.push(item.clone());
        item
    }

    /// Alloue et initialise l'objet `MenuTarget` avec les éléments de menu à
    /// mettre à jour. Renvoie un objet Objective-C prêt à recevoir les actions.
    fn creer_cible(
        mtm: MainThreadMarker,
        item_panneau: Retained<NSMenuItem>,
        themes: [Retained<NSMenuItem>; 3],
    ) -> Retained<MenuTarget> {
        let alloue = MenuTarget::alloc(mtm).set_ivars(MenuTargetIvars {
            item_panneau,
            themes,
        });
        unsafe { msg_send![super(alloue), init] }
    }

    /// Verrouille la file des actions et renvoie la garde pour y écrire.
    fn file_des_actions() -> MutexGuard<'static, Vec<ActionMenu>> {
        ACTIONS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Variables d'instance de la cible : les éléments de menu sensibles à
    /// l'état (panneau et thèmes).
    struct MenuTargetIvars {
        item_panneau: Retained<NSMenuItem>,
        themes: [Retained<NSMenuItem>; 3],
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[ivars = MenuTargetIvars]
        struct MenuTarget;

        impl MenuTarget {
            /// Les éléments de menu restent toujours actifs (jamais grisés).
            #[unsafe(method(validateMenuItem:))]
            fn valider_menu_item(&self, _item: &NSMenuItem) -> bool {
                true
            }

            /// Reçoit le clic sur un élément de menu, identifie l'élément par
            /// son `tag`, enfile l'action correspondante et force une reprise
            /// de la peinture egui.
            #[unsafe(method(handleMenuAction:))]
            fn handle_menu_action(&self, sender: Option<&AnyObject>) {
                let Some(sender) = sender else {
                    return;
                };
                let tag: NSInteger = unsafe { msg_send![sender, tag] };
                match tag {
                    TAG_PANNEAU => {
                        file_des_actions().push(ActionMenu::BasculerPanneau);
                    }
                    TAG_THEME_SOMBRE | TAG_THEME_CLAIR | TAG_THEME_CONTRASTE => {
                        let theme = match tag {
                            TAG_THEME_SOMBRE => Theme::Sombre,
                            TAG_THEME_CLAIR => Theme::Clair,
                            _ => Theme::Contraste,
                        };
                        self.appliquer_theme(theme);
                        file_des_actions().push(ActionMenu::Theme(theme));
                    }
                    TAG_NOUVEAU => file_des_actions().push(ActionMenu::Nouveau),
                    TAG_OUVRIR => file_des_actions().push(ActionMenu::Ouvrir),
                    TAG_ENREGISTRER => file_des_actions().push(ActionMenu::Enregistrer),
                    TAG_ENREGISTRER_SOUS => file_des_actions().push(ActionMenu::EnregistrerSous),
                    TAG_QUITTER => file_des_actions().push(ActionMenu::Quitter),
                    TAG_REDUIRE | TAG_AGRANDIR | TAG_PLEIN_ECRAN | TAG_FERMER_FENETRE => {
                        self.action_fenetre(tag)
                    }
                    _ => {}
                }
                // Sans cela, egui ne repaint pas après un clic dans le menu
                // natif (aucun événement n'atteint la fenêtre) et l'effet de
                // l'action ne s'affiche pas tant que la souris n'a pas bougé.
                if let Some(ctx) = CTX.get() {
                    ctx.request_repaint();
                }
            }
        }
    );

    impl MenuTarget {
        /// Cocher uniquement l'élément de menu du thème actif.
        fn appliquer_theme(&self, theme: Theme) {
            let actifs = [
                theme == Theme::Sombre,
                theme == Theme::Clair,
                theme == Theme::Contraste,
            ];
            for (item, actif) in self.ivars().themes.iter().zip(actifs) {
                item.setState(if actif {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
        }

        /// Applique les commandes de fenêtre natives (réduire, agrandir, plein
        /// écran, fermer) via les commandes de fenêtre egui.
        fn action_fenetre(&self, tag: NSInteger) {
            let Some(ctx) = CTX.get() else {
                return;
            };
            match tag {
                TAG_REDUIRE => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                TAG_AGRANDIR => ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true)),
                TAG_PLEIN_ECRAN => {
                    let plein = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!plein));
                }
                TAG_FERMER_FENETRE => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                _ => {}
            }
        }
    }

    /// Change le libellé de l'élément « Panneau » en fonction de la position.
    pub fn mettre_a_jour_panneau(gauche: bool) {
        let Some(target) = TARGET.get().map(|p| unsafe { &*p.0 }) else {
            return;
        };
        let titre = if gauche { "Panneau à gauche" } else { "Panneau à droite" };
        target.ivars().item_panneau.setTitle(&NSString::from_str(titre));
    }

    /// Met à jour la coche du thème actif dans le menu.
    pub fn mettre_a_jour_theme(theme: Theme) {
        let Some(target) = TARGET.get().map(|p| unsafe { &*p.0 }) else {
            return;
        };
        target.appliquer_theme(theme);
    }
}
