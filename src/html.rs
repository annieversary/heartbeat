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

.active {
    color: #1da23e;
}
.inactive {
    color: #d90422;
}

.absences, .recent-beats {
    width: 80vw;
    display: flex;
    flex-direction: row;
}

.right {
    flex: 1;
}
.left .line {
    margin-right: 1rem;
    border-right: 1px solid #ffa2da;
}

.line {
    display: flex;
    flex-direction: row;
    width: 100%;
    height: 1rem;
    position: relative;
}

.line:nth-child(1) {
    margin-bottom: 10px;
    border-right: unset;
}
.line:nth-child(2) {
    border-top: 1px solid #ffa2da;
}

.line span {
    position: absolute;
}

.absences span.hours {
    position: absolute;
    width: calc(100% / 24);
    text-align: center;
    color: #e3228f;
}

.line span.dots {
    width: 1px;
    height: 1rem;
    background-color: #ffa2da;
}

.absences .line span.start,
.absences .line span.end {
    width: 8px;
    height: 1rem;
}
.absences .line span.start {
    background-color: #ac3333;
}
.absences .line span.start:hover {
    background-color: #c94949;
}
.absences .line span.end {
    background-color: #9a37ec;
}
.absences .line span.end:hover {
    background-color: #c737ec;
}
.absences .line span.length {
    background-color: #8000806e;
    height: 1rem;
    transition: 200ms;
}
.absences .line span.length:hover {
    background-color: #d715d76e;
}

.recent-beats .beat {
    width: 1px;
    height: 1rem;
    background-color: #800080;
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
