# Web sample

First, build wasm modules by running the following in the repo root:

```
./build_extras.sh
```

Run the following in this directory:

```
python3 -m http.server
```

Going to the hosted URL, you should see a cube being drawn. Clicking and dragging your mouse should
orbit around the cube. Scrolling (or pinching) should enable zooming and in out of the view.
