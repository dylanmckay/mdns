error_chain! {
    types {
        Error, ErrorKind, ResultExt;
    }

    foreign_links {
        Io(::std::io::Error);
        Dns(crate::dns::Error);
    }
}
