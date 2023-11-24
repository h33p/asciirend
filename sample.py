import asciirend as ar

# Dumped from the javascript sample (back when it had spinning cube)
scene_desc = '{"camera":{"transform":[0.8660253,0.49999997,-7.450581e-9,0.0,-0.4330127,0.7499999,-0.49999997,0.0,-0.25,0.4330127,0.8660253,0.0,0.4330127,-0.75,0.49999997,1.0],"proj":[1.104,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,-0.0010000101,0.0,0.0,0.0,-0.00001001358,1.0]},"camera_controller":{"fov_y":1.0,"focus_point":[0.0,0.0,0.0],"rot":[-0.25,-0.0669873,0.25,0.93301266],"dist":1.0,"in_motion":"None","scroll_sensitivity":0.02,"orbit_sensitivity":1.0,"last_down":false,"pressed":false},"objects":[{"transform":[0.91837555,-0.39570975,0.0,0.0,0.39570975,0.91837555,0.0,0.0,0.0,0.0,0.99999994,0.0,0.0,0.0,0.0,1.0],"material":1,"ty":{"Cube":{"size":[1.0,1.0,1.0]}},"text":"Test"}],"bg":{"color":[0.01,0.01,0.01]},"dithering":{"count_frames":false,"frame_cnt":4181}}'

scale = 2
w, h = 32 * scale, 9 * scale

scene = ar.ascii_render(scene_desc, 0, w, h, 16 / 9, True, 1.0, 0.01, 100.0)
print(scene)
