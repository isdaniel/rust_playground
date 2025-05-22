use trpl::Html;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    trpl::run(async {
        let url = &args[1];
        let title = page_title(url).await;
        println!("{:?}", title);
    })
}

async fn page_title(url:&str) -> Option<String>{
    let html: String = trpl::get(url).await.text().await;
    Html::parse(&html)
        .select_first("title")
        .map(| e| e.inner_html())
}