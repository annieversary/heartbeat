use maud::{html, PreEscaped};

const CSS: &str = r#"
html {
    font-family: 'Chivo Mono', monospace;
    font-weight: 300;
    background-color: #ffd1dc;
}

li {
    list-style: none;
}

.small {
    font-size: 0.7rem;
}

.graph svg {
    height: 90vh;
}
"#;

pub fn base_template(content: PreEscaped<String>) -> PreEscaped<String> {
    html! {
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";

                title {"annie's heartbeat"}

                // TODO og meta tags

                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link href="https://fonts.googleapis.com/css2?family=Chivo+Mono:ital,wght@0,200;0,300;0,400;0,700;1,200;1,300;1,400;1,700&display=swap" rel="stylesheet";
                link href="https://fonts.googleapis.com/css2?family=Inconsolata&display=swap" rel="stylesheet";
                style {(CSS)}
            }
            body {
                (content)
            }
        }
    }
}
