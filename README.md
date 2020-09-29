This library allows parsing and writing [.po translation files](https://www.gnu.org/software/gettext/manual/html_node/PO-Files.html).  
It works with files considered valid by `msgfmt`, e.g. `msgfmt -c example.po`.

The following work needs to be done:
* Add unit tests to cover parsing various PO formats
* Refactor code such that written strings wrap across multiple lines
* Maybe add minimal validation somehow rather than assuming PO files match the spec