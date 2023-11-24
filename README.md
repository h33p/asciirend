# asciirend

```text
. ...    . . .  .  .     . . ..   ..  .. .  ..    .  . .. ... . . ...
.     . .       ..  ..  ...... . .   .......   .  ..  . .. ....... . ..
.  .   .   ... .. . =======+=+==+======+=====+======.  . ...            .
.   ....######======+================+=====+===+===+===+=+=+=+== .... ...
    . . .###%##########=============+===+=========++++++=====+      . . .
.     .  ..######%###%###%####%======+===++==++=+===+==+++++=..  .   . .
 ...    ... ##########%##%######%####=+==+======++++===+==+=. .  .  ...
  .. ..   . .#####%#######%####...........=+====+=====+=== ..    . .. ..
. ..... .  ... ##%####%%######%.asciirend.======+=+==+===  . .... ..... .
  ..    .. . .  ##########%####...........+========+====     ..    ...  .
. .    ..  .     ##%##############%##=+++======+==+=== .. ... ..  .. ..
 .. ....  .       .##################=+=====++===+==+   .   .      .  .
.      ...  . ...    #%####%#####%###==+=+==+=+++==    ...  . ...      ..
. .. . . ...  . ..  .. ##########%%##+===++===+=+  .  .   ..    ..  ..
.    .    . .. . .. .    ############+===+=++==.    . . ..  .  ..    .
 . .   .  . .   .  . ...   ##########+==++=++ ..... . . . .  .      .   .
 ....  ..    ... ....  ..    #%#####%==+===   .. .    .  .. .. . ....  ..
..   . ... .. .. ..     ...  ..##%###+=== .    ...   .  ..    .   .   . .
```

## Generic ascii renderer

`asciirend` is a `no_std` compatible 3D rendering core. This crate renders objects in several
stages:

- Primitive shading (similar to vertex shading, albeit works on whole primitives).
- Fragment shading.
- Additional text pass.

Fragments are shaded in full sRGB float color space, which then is quantized to characters with a
dithering algorithm. This allows the image to have clearer color transitions, as opposed to direct
character shading. This also means the rendering backend is agnostic to the available color space
(so long as it's not HDR!), which is important for a generic `no_std` compatible renderer.

## Examples

Please see [`examples/sample.rs`](examples/sample.rs) for usage sample.

### JavaScript

`asciirend` has a javascript sample that uses WASM. Albeit the API surface is limited, it is 
possible to use it to draw basic interactive scenes. Please see [`web_sample`](web_sample/)
directory for details.

### Python

In addition, there is a python sample (and pip package). Please see [`python`](python/)
subdirectory for more details, or install `asciirend` package, and run the python sample at
[`sample.py`](sample.py).
