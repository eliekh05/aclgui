use crate::app::AclApp;

pub fn draw(app: &mut AclApp, ui: &mut egui::Ui) {
    let available = ui.available_size();

    // Message list
    let list_height = available.y - 60.0;
    egui::ScrollArea::vertical()
        .max_height(list_height)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for msg in &app.chat_history {
                if msg.from_user {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.group(|ui| {
                            ui.set_max_width(available.x * 0.7);
                            ui.label(&msg.text);
                        });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(80, 160, 240), "🤖");
                        ui.group(|ui| {
                            ui.set_max_width(available.x * 0.8);
                            ui.label(&msg.text);
                        });
                    });
                }
                ui.add_space(4.0);
            }
        });

    ui.separator();

    // Input row
    ui.horizontal(|ui| {
        let input_response = ui.add(
            egui::TextEdit::singleline(&mut app.chat_input)
                .hint_text("Ask about this path's permissions…")
                .desired_width(available.x - 80.0),
        );
        let send = ui.button("Send");
        if send.clicked()
            || (input_response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            app.send_chat();
        }
        // Keep focus on input after send
        input_response.request_focus();
    });
}
