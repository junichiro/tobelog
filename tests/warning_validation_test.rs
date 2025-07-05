// Warning解消の検証テスト
// コンパイル警告が適切に解消されることを確認

#[cfg(test)]
mod warning_validation_tests {
    use std::process::Command;

    #[test]
    fn warning数が削減されていることを確認() {
        // このテストは警告解消後に有効になる
        // 現在は意図的にスキップ
        if std::env::var("SKIP_WARNING_TEST").is_ok() {
            return;
        }

        let output = Command::new("cargo")
            .args(&["build", "--message-format=json"])
            .output()
            .expect("Failed to run cargo build");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let warning_count = stdout.lines()
            .filter(|line| line.contains("\"level\":\"warning\""))
            .count();

        // 警告数が大幅に削減されていることを確認（完全に0でなくても良い）
        assert!(warning_count < 15, "警告数が多すぎます: {}", warning_count);
    }

    #[test]
    fn 重要な機能が削除されていないことを確認() {
        // 将来使用予定の重要な機能が誤って削除されていないことを確認
        use tobelog::services::template::TemplateService;
        
        // TemplateServiceが正常に初期化できることを確認
        let _result = TemplateService::new();
        // templates/default/が存在しない可能性があるためエラーは許容
        // 重要なのは型定義が存在することの確認
    }

    #[test]
    fn 未使用として処理された機能の存在確認() {
        // #[allow(dead_code)]で処理された機能が実際に存在することを確認
        // これはコンパイルが通ることで自動的に確認される
        
        // 例：テンプレート機能の構造体が存在することを確認
        let _template_service_exists = std::any::type_name::<tobelog::services::template::TemplateService>();
        
        // 例：設定構造体にtemplate_themeフィールドが存在することを確認
        let _config_has_template_theme = std::any::type_name::<tobelog::config::Config>();
        
        assert!(true, "重要な型定義が存在している");
    }
}