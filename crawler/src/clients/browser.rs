// https://commondatastorage.googleapis.com/chromium-browser-snapshots/index.html?prefix=Linux_x64/
// https://commondatastorage.googleapis.com/chromium-browser-snapshots/index.html?prefix=Linux_ARM_Cross-Compile/
// https://storage.googleapis.com/chromium-browser-snapshots/Linux_x64/1711941/chrome-linux.zip
// https://storage.googleapis.com/chrome-for-testing-public/Linux_x64/1711941/chrome-linux.zip

mod test {
    use std::time::Duration;

    use chromiumoxide::{
        Browser, BrowserConfig, cdp::browser_protocol::page::CaptureScreenshotFormat,
        page::ScreenshotParams,
    };
    use futures::StreamExt;
    use tokio::time::{MissedTickBehavior, interval};

    #[tokio::test]
    async fn test_browser() {
        println!("starting chrome");
        let (mut browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                // .chrome_executable("/home/lucas/project-carbon/backend/chrome/chrome-linux/chrome")
                .chrome_executable("/home/lucas/project-carbon/backend/chrome/chrome-154/chrome")
                .with_head()
                .hide()
                .window_size(1680, 1050)
                .viewport(None)
                .disable_default_args()
                .arg("no-first-run")
                .arg(("lang", "en_US"))
                // .headless_mode(HeadlessMode::New)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        println!("loading page");
        let page = browser.new_page("about:blank").await.unwrap();
        // page.enable_stealth_mode_with_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/154.0.0.0 Safari/537.36").await.unwrap();

        page.goto("https://latulippe.com/en/catalog/shooting/hunting-rifles/")
            .await
            .unwrap();

        page.wait_for_navigation().await.unwrap();

        println!("save page");
        // page.save_screenshot(
        //     ScreenshotParams::builder()
        //         .format(CaptureScreenshotFormat::Png)
        //         .full_page(true)
        //         .omit_background(false)
        //         .build(),
        //     "/home/lucas/project-carbon/backend/chrome/output5.png",
        // )
        // .await
        // .unwrap();
        let out_dir = "/home/lucas/project-carbon/backend/chrome/frames";
        tokio::fs::create_dir_all(out_dir).await.unwrap();

        let mut tick = interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        for i in 0..10 {
            tick.tick().await;
            page.save_screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build(),
                format!("{out_dir}/frame_{i:04}.png"),
            )
            .await
            .unwrap();
        }

        println!("killing");
        browser.close().await.unwrap();
        handle.await.unwrap();
    }
}
