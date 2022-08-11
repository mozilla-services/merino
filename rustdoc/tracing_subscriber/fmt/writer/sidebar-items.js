window.SIDEBAR_ITEMS = {"enum":[["EitherWriter","A writer that is one of two types implementing `io::Write`."]],"struct":[["ArcWriter","Implements [`std::io::Write`] for an [`Arc`] where `&W: Write`."],["BoxMakeWriter","A writer that erases the specific `io::Write` and [`MakeWriter`] types being used."],["OrElse","Combines a [`MakeWriter`] that returns an [`OptionalWriter`] with another [`MakeWriter`], so that the second [`MakeWriter`] is used when the first [`MakeWriter`] returns [`OptionalWriter::none`]."],["Tee","Combines two types implementing [`MakeWriter`] (or [`std::io::Write`]) to produce a writer that writes to both [`MakeWriter`]’s returned writers."],["TestWriter","A writer intended to support `libtest`’s output capturing for use in unit tests."],["WithFilter","A [`MakeWriter`] combinator that wraps a [`MakeWriter`] with a predicate for span and event `Metadata`, so that the [`MakeWriterExt::make_writer_for`] method returns [`OptionalWriter::some`] when the predicate returns `true`, and [`OptionalWriter::none`] when the predicate returns `false`."],["WithMaxLevel","A [`MakeWriter`] combinator that only returns an enabled writer for spans and events with metadata at or below a specified verbosity `Level`."],["WithMinLevel","A [`MakeWriter`] combinator that only returns an enabled writer for spans and events with metadata at or above a specified verbosity `Level`."]],"trait":[["MakeWriter","A type that can create `io::Write` instances."],["MakeWriterExt","Extension trait adding combinators for working with types implementing [`MakeWriter`]."]],"type":[["OptionalWriter","A writer which may or may not be enabled."]]};