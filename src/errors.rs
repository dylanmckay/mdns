error_chain! {
    types {
        Error, ErrorKind;
    }

    foreign_links {
        ::std::io::Error, Io;
        ::dns::Error, Dns;
    }
}
