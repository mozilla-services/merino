(function() {var implementors = {};
implementors["actix_http"] = [{"text":"impl <a class=\"trait\" href=\"actix_service/trait.Service.html\" title=\"trait actix_service::Service\">Service</a>&lt;<a class=\"struct\" href=\"actix_http/struct.Request.html\" title=\"struct actix_http::Request\">Request</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.62.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.62.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"futures_core/stream/trait.Stream.html\" title=\"trait futures_core::stream::Stream\">Stream</a>&lt;Item = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.62.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"bytes/bytes/struct.Bytes.html\" title=\"struct bytes::bytes::Bytes\">Bytes</a>, <a class=\"enum\" href=\"actix_http/error/enum.PayloadError.html\" title=\"enum actix_http::error::PayloadError\">PayloadError</a>&gt;&gt; + 'static, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.62.0/alloc/alloc/struct.Global.html\" title=\"struct alloc::alloc::Global\">Global</a>&gt;&gt;&gt;&gt; for <a class=\"struct\" href=\"actix_http/h1/struct.ExpectHandler.html\" title=\"struct actix_http::h1::ExpectHandler\">ExpectHandler</a>","synthetic":false,"types":["actix_http::h1::expect::ExpectHandler"]},{"text":"impl&lt;T&gt; <a class=\"trait\" href=\"actix_service/trait.Service.html\" title=\"trait actix_service::Service\">Service</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.62.0/std/primitive.tuple.html\">(</a><a class=\"struct\" href=\"actix_http/struct.Request.html\" title=\"struct actix_http::Request\">Request</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.62.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.62.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"futures_core/stream/trait.Stream.html\" title=\"trait futures_core::stream::Stream\">Stream</a>&lt;Item = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.62.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"bytes/bytes/struct.Bytes.html\" title=\"struct bytes::bytes::Bytes\">Bytes</a>, <a class=\"enum\" href=\"actix_http/error/enum.PayloadError.html\" title=\"enum actix_http::error::PayloadError\">PayloadError</a>&gt;&gt; + 'static, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.62.0/alloc/alloc/struct.Global.html\" title=\"struct alloc::alloc::Global\">Global</a>&gt;&gt;&gt;, <a class=\"struct\" href=\"actix_codec/framed/struct.Framed.html\" title=\"struct actix_codec::framed::Framed\">Framed</a>&lt;T, <a class=\"struct\" href=\"actix_http/h1/struct.Codec.html\" title=\"struct actix_http::h1::Codec\">Codec</a>&gt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.62.0/std/primitive.tuple.html\">)</a>&gt; for <a class=\"struct\" href=\"actix_http/h1/struct.UpgradeHandler.html\" title=\"struct actix_http::h1::UpgradeHandler\">UpgradeHandler</a>","synthetic":false,"types":["actix_http::h1::upgrade::UpgradeHandler"]}];
implementors["actix_service"] = [];
implementors["actix_web"] = [];
implementors["merino_web"] = [{"text":"impl&lt;S, B&gt; <a class=\"trait\" href=\"actix_service/trait.Service.html\" title=\"trait actix_service::Service\">Service</a>&lt;<a class=\"struct\" href=\"actix_web/service/struct.ServiceRequest.html\" title=\"struct actix_web::service::ServiceRequest\">ServiceRequest</a>&gt; for <a class=\"struct\" href=\"merino_web/middleware/metrics/struct.MetricsMiddleware.html\" title=\"struct merino_web::middleware::metrics::MetricsMiddleware\">MetricsMiddleware</a>&lt;S&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;S: <a class=\"trait\" href=\"actix_service/trait.Service.html\" title=\"trait actix_service::Service\">Service</a>&lt;<a class=\"struct\" href=\"actix_web/service/struct.ServiceRequest.html\" title=\"struct actix_web::service::ServiceRequest\">ServiceRequest</a>, Response = <a class=\"struct\" href=\"actix_web/service/struct.ServiceResponse.html\" title=\"struct actix_web::service::ServiceResponse\">ServiceResponse</a>&lt;B&gt;&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;B: 'static + <a class=\"trait\" href=\"actix_http/body/message_body/trait.MessageBody.html\" title=\"trait actix_http::body::message_body::MessageBody\">MessageBody</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;S::<a class=\"associatedtype\" href=\"actix_service/trait.Service.html#associatedtype.Future\" title=\"type actix_service::Service::Future\">Future</a>: 'static,<br>&nbsp;&nbsp;&nbsp;&nbsp;S::<a class=\"associatedtype\" href=\"actix_service/trait.Service.html#associatedtype.Error\" title=\"type actix_service::Service::Error\">Error</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.62.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,&nbsp;</span>","synthetic":false,"types":["merino_web::middleware::metrics::MetricsMiddleware"]},{"text":"impl&lt;S, B&gt; <a class=\"trait\" href=\"actix_service/trait.Service.html\" title=\"trait actix_service::Service\">Service</a>&lt;<a class=\"struct\" href=\"actix_web/service/struct.ServiceRequest.html\" title=\"struct actix_web::service::ServiceRequest\">ServiceRequest</a>&gt; for <a class=\"struct\" href=\"merino_web/middleware/sentry/struct.SentryMiddleware.html\" title=\"struct merino_web::middleware::sentry::SentryMiddleware\">SentryMiddleware</a>&lt;S&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;S: <a class=\"trait\" href=\"actix_service/trait.Service.html\" title=\"trait actix_service::Service\">Service</a>&lt;<a class=\"struct\" href=\"actix_web/service/struct.ServiceRequest.html\" title=\"struct actix_web::service::ServiceRequest\">ServiceRequest</a>, Response = <a class=\"struct\" href=\"actix_web/service/struct.ServiceResponse.html\" title=\"struct actix_web::service::ServiceResponse\">ServiceResponse</a>&lt;B&gt;, Error = <a class=\"struct\" href=\"actix_web/error/error/struct.Error.html\" title=\"struct actix_web::error::error::Error\">ActixError</a>&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;B: 'static + <a class=\"trait\" href=\"actix_http/body/message_body/trait.MessageBody.html\" title=\"trait actix_http::body::message_body::MessageBody\">MessageBody</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;S::<a class=\"associatedtype\" href=\"actix_service/trait.Service.html#associatedtype.Future\" title=\"type actix_service::Service::Future\">Future</a>: 'static,<br>&nbsp;&nbsp;&nbsp;&nbsp;B::<a class=\"associatedtype\" href=\"actix_http/body/message_body/trait.MessageBody.html#associatedtype.Error\" title=\"type actix_http::body::message_body::MessageBody::Error\">Error</a>: <a class=\"trait\" href=\"actix_web/error/response_error/trait.ResponseError.html\" title=\"trait actix_web::error::response_error::ResponseError\">ResponseError</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;S::<a class=\"associatedtype\" href=\"actix_service/trait.Service.html#associatedtype.Error\" title=\"type actix_service::Service::Error\">Error</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.62.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,&nbsp;</span>","synthetic":false,"types":["merino_web::middleware::sentry::SentryMiddleware"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()