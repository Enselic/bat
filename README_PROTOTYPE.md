THIS IS JUST A SKETCH


This is a prototype that experiements with one way to solve https://github.com/sharkdp/bat/issues/951.

What works:
* All regression tests pass
* Significantly faster startup speed for small syntaxes, but also improvments for large syntaxes that embemds other syntaxes

What does not work:
* lookup by first line
* lookup by file extension
* Size of bat binary significantly larger
* Loading syntaxes from a user-cache
* Public API compatiblity
* metadata on new assets


Future considerations:
* We might want to load ThemeSet cleverly too, in which case the current lookup data structure is insufficent, so would be good not to expose it publicly if possible

MVP scope:
* Only simple lookup by name and extension. Later versions can support first line match for example.

The way it works:
* We analyzes dependencies between SyntaxDefinitions and write them to file with lookup map
* TODO

syntaxes.bin is changed to contain several independent syntax sets, concatenated in binary form and looked up with an offset and size hashmap.
