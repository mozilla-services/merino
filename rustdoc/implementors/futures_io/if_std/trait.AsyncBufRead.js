(function() {var implementors = {};
implementors["futures_lite"] = [{"text":"impl&lt;R:&nbsp;<a class=\"trait\" href=\"futures_io/if_std/trait.AsyncRead.html\" title=\"trait futures_io::if_std::AsyncRead\">AsyncRead</a>&gt; <a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a> for <a class=\"struct\" href=\"futures_lite/io/struct.BufReader.html\" title=\"struct futures_lite::io::BufReader\">BufReader</a>&lt;R&gt;","synthetic":false,"types":["futures_lite::io::BufReader"]},{"text":"impl&lt;T&gt; <a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a> for <a class=\"struct\" href=\"futures_lite/io/struct.Cursor.html\" title=\"struct futures_lite::io::Cursor\">Cursor</a>&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.58.0/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.58.0/std/primitive.slice.html\">[</a><a class=\"primitive\" href=\"https://doc.rust-lang.org/1.58.0/std/primitive.u8.html\">u8</a><a class=\"primitive\" href=\"https://doc.rust-lang.org/1.58.0/std/primitive.slice.html\">]</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.58.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,&nbsp;</span>","synthetic":false,"types":["futures_lite::io::Cursor"]},{"text":"impl <a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a> for <a class=\"struct\" href=\"futures_lite/io/struct.Empty.html\" title=\"struct futures_lite::io::Empty\">Empty</a>","synthetic":false,"types":["futures_lite::io::Empty"]},{"text":"impl&lt;R:&nbsp;<a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a>&gt; <a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a> for <a class=\"struct\" href=\"futures_lite/io/struct.Take.html\" title=\"struct futures_lite::io::Take\">Take</a>&lt;R&gt;","synthetic":false,"types":["futures_lite::io::Take"]},{"text":"impl&lt;R1:&nbsp;<a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a>, R2:&nbsp;<a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a>&gt; <a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a> for <a class=\"struct\" href=\"futures_lite/io/struct.Chain.html\" title=\"struct futures_lite::io::Chain\">Chain</a>&lt;R1, R2&gt;","synthetic":false,"types":["futures_lite::io::Chain"]}];
implementors["sluice"] = [{"text":"impl <a class=\"trait\" href=\"futures_io/if_std/trait.AsyncBufRead.html\" title=\"trait futures_io::if_std::AsyncBufRead\">AsyncBufRead</a> for <a class=\"struct\" href=\"sluice/pipe/struct.PipeReader.html\" title=\"struct sluice::pipe::PipeReader\">PipeReader</a>","synthetic":false,"types":["sluice::pipe::PipeReader"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()