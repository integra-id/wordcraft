egui's default faces, from `crates/epaint_default_fonts/fonts` of the pinned egui revision, with
their licences. The web viewer fetches them after it starts (`web::EGUI_FACE_FILES`) instead of
compiling them in (the `egui-fonts` feature of `wordcraft-ui-egui`).
