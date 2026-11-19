#!/usr/bin/env python3
"""Offline screen-space detail budget for the authored orthographic city camera.

Uses real GLB bounds and audited physical dimensions. This is a projection study,
not a GPU time benchmark or a runtime LOD/culling implementation.
"""
import argparse
import importlib.util
import json
import math
from pathlib import Path
import struct


def assess(width, height):
    folder = Path(__file__).resolve().parents[1]/'assets'
    data = (folder/'city-block.glb').read_bytes()
    length = struct.unpack_from('<I', data, 12)[0]
    gltf = json.loads(data[20:20+length])
    # The original generator bakes all transforms into positions. Reject future
    # transformed assets rather than silently using the wrong framing radius.
    if any(any(key in node for key in ['matrix', 'translation', 'rotation', 'scale']) for node in gltf['nodes']):
        raise ValueError('Projection study requires the current baked-transform city asset')
    boxes = [gltf['accessors'][p['attributes']['POSITION']]
             for mesh in gltf['meshes'] for p in mesh['primitives']]
    low = [min(b['min'][axis] for b in boxes) for axis in range(3)]
    high = [max(b['max'][axis] for b in boxes) for axis in range(3)]
    radius = math.sqrt(sum((a-b)**2 for a, b in zip(high, low)))/2
    spec = importlib.util.spec_from_file_location('audit', folder/'check_city_block.py')
    audit = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(audit)
    physical = audit.check()
    person = sum(physical['adult_height_metres'])/2
    tree = sum(physical['tree_height_metres'])/2
    car = physical['vehicle_dimensions']['car']['length_metres']
    pitch = math.atan(0.35)+0.24
    yaw = 0.55
    # Project unit world axes into the screen plane; roads run along X or Z.
    horizontal_axes = [math.hypot(math.cos(yaw), math.sin(yaw)*math.sin(pitch)),
                       math.hypot(math.sin(yaw), math.cos(yaw)*math.sin(pitch))]
    rows = []
    for label, ratio in [('放大 2 倍', 0.5), ('默认', 1), ('缩小 2 倍', 2), ('缩小 4 倍', 4), ('缩小 8 倍', 8)]:
        zoom = max(0.2, min(8, 0.56*ratio))
        pixels_per_unit = height/(2*radius*1.3*zoom/min(1, width/height))
        vertical = pixels_per_unit*math.cos(pitch)/4
        # Original atlas glass is 92/128 of a 2 m bay, 86/128 of a 3.2 m story.
        glass_width = 2*92/128
        glass_height = 3.2*86/128
        rows.append(dict(view=label, camera_zoom=zoom,
            adult_vertical_pixels=person*vertical,
            tree_vertical_pixels=tree*vertical,
            car_length_pixels_range=[car/4*pixels_per_unit*f for f in sorted(horizontal_axes)],
            window_glass_width_pixels_range=[glass_width/4*pixels_per_unit*f for f in sorted(horizontal_axes)],
            window_glass_height_pixels=glass_height*vertical))
    return dict(render_target_pixels=[width,height], scene_units_metres=4,
        glb_bounds=[low,high],framing_radius=radius,default_camera_zoom=0.56,
        projection='orthographic, yaw 0.55, pitch atan(0.35)+0.24, identity model rotation',
        samples=rows,
        candidate_thresholds={'window_min_axis_pixels':{'room_actor':12,'curtain_shape':6,'folds':20},
            'pedestrian_vertical_pixels':{'hide_below':3,'fade_interval':[3,6]},
            'vehicle_length_pixels':{'hide_below':4,'simple_silhouette_below':16},
            'tree_vertical_pixels':{'hide_below':4,'static_silhouette_below':12}},
        scope='Calculated physical-pixel feature spans at the authored camera; excludes occlusion and submesh shape. Thresholds are proposals requiring native visual/GPU validation, not active LOD. Counts/visibility changes must use hysteresis or fades; transparent hiding alone does not remove vertex work.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--width',type=int,default=2960)
    parser.add_argument('--height',type=int,default=1362)
    parser.add_argument('--output',type=Path,required=True)
    args = parser.parse_args()
    if min(args.width,args.height)<=0:
        parser.error('render size must be positive')
    result=assess(args.width,args.height)
    args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(result,ensure_ascii=False,indent=2))
