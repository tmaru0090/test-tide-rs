// rustでwebフロントのなんか
// 適当にpythonコードをweb上で実行するやつ
//
mod alias;
use alias::Res;
use async_std::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tide::{Body, Request, Response, Server, StatusCode};

use pyo3::types::PyDict;

use pyo3::prelude::*;
use pyo3::types::IntoPyDict;

#[derive(Serialize, Deserialize, Clone)]
struct FormQuery {
    code: String,
}

#[async_std::main]
async fn main() -> Res<()> {
    // pythonの非同期初期化
    pyo3::prepare_freethreaded_python();
    let state = Arc::new(Mutex::new(FormQuery {
        code: String::new(),
    }));

    let mut app = tide::with_state(state.clone());
    //Server::<Arc<Mutex<FormQuery>>>::with_state(state.clone());

    app.at("/resources/").serve_dir("resources/")?;
    app.at("/").serve_file("html/index.html")?;
    let mut form_q = FormQuery {
        code: String::new(),
    };
    app.at("/request/")
        .get(|req: tide::Request<Arc<Mutex<FormQuery>>>| async move {
            let q: FormQuery = req.query()?;
            let mut state = req.state().lock().await;
            let mut output = String::new();
            *state = q.clone();
            let mut err_message = String::new();
            /*
                        Python::with_gil(|py| {
                            let locals = PyDict::new(py);
                            locals.set_item("sys", py.import("sys").unwrap()).unwrap();

                            py.run(
                                r#"
            import io
            sys.stdout = io.StringIO()
            "#,
                                None,
                                Some(locals),
                            )
                            .unwrap();

                            if let Some(sys_module) = locals.get_item("sys").expect("Failed to get 'sys' module"){
                                let stdout = sys_module.getattr("stdout").unwrap();
                                output = stdout.call_method0("getvalue").unwrap().extract().unwrap();
                                println!("{}",output);
                                if let Err(err) = py.run(&*state.code.clone(), None, Some(locals)) {
                                    err_message = err.to_string();
                                }
                            } else {
                                err_message = "Failed to get 'sys' module".to_string();
                            }
                        });
                        */

            Python::with_gil(|py| {
                let locals = PyDict::new(py);
                let original_stdout = py.import("sys").unwrap().getattr("stdout").unwrap();
                locals.set_item("sys", py.import("sys").unwrap()).unwrap();
                locals.set_item("time", py.import("time").unwrap()).unwrap();
                locals.set_item("random", py.import("random").unwrap()).unwrap();
                py.run(
                    r#"
import io
sys.stdout = io.StringIO()
"#,
                    None,
                    Some(locals),
                )
                .unwrap();

                // Pythonコードの実行
                if let Err(err) = py.run(&*state.code.clone(), None, Some(locals)) {
                    err_message = err.to_string();
                }

                // StringIOバッファの内容を取得
                let stdout = locals
                    .get_item("sys")
                    .unwrap()
                    .ok_or("")
                    .unwrap()
                    .getattr("stdout")
                    .unwrap();
                output = stdout.call_method0("getvalue").unwrap().extract().unwrap();

                // 元のstdoutに戻す
                py.import("sys")
                    .unwrap()
                    .setattr("stdout", original_stdout)
                    .unwrap();

                // 出力内容を表示
                println!("{}", output);
            });

            if !err_message.is_empty() {
                Ok(tide::Response::builder(200)
                    .body(format!(
                        "<html><h1>エラー: </h1><h3>{}</h3></html>",
                        err_message
                    ))
                    .header("Server", "tide")
                    .content_type(tide::http::mime::HTML)
                    .build())
            } else {
                Ok(tide::Response::builder(200)
                    .body(format!(
                        "<html><h1>実行結果:</h1><h3>{}</h3></html>",
                        output
                    ))
                    .header("Server", "tide")
                    .content_type(tide::http::mime::HTML)
                    .build())
            }
        });
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
