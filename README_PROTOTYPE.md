THIS IS JUST A SKETCH


This is a prototype that experiements with one way to solve https://github.com/sharkdp/bat/issues/951.

What works:
* All regression tests pass
* Significantly faster startup speed for small syntaxes, but also improvments for large syntaxes that embemds other syntaxes

What does not work:
* Size of bat binary significantly larger
* Loading syntaxes from a user-cache
* Public API compatiblity
* 
