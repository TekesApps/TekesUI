"""Check exported geometry against physical scale, without Pillow or the runtime."""
from pathlib import Path
import json
import struct

ROOT = Path(__file__).parent
METRES = 4.0


def check():
    data = (ROOT / 'city-block.glb').read_bytes()
    size = struct.unpack_from('<I', data, 12)[0]
    gltf = json.loads(data[20:20 + size])
    binary = data[28 + size:]
    recipe = json.loads((ROOT / 'city-block.json').read_text())

    def values(index):
        accessor = gltf['accessors'][index]
        view = gltf['bufferViews'][accessor['bufferView']]
        dims = {'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[accessor['type']]
        start = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
        packed = struct.unpack_from('<' + 'f' * accessor['count'] * dims, binary, start)
        return [packed[i:i + dims] for i in range(0, len(packed), dims)]

    def primitives(name):
        node = next(n for n in gltf['nodes'] if n['name'] == name)
        return gltf['meshes'][node['mesh']]['primitives']

    def bounds(names):
        points = [p for name in names for prim in primitives(name)
                  for p in values(prim['attributes']['POSITION'])]
        return [(min(p[i] for p in points), max(p[i] for p in points)) for i in range(3)]

    screen=next(n for n in gltf['nodes'] if n['name']=='construction-dust-screen')
    assert screen['extras']['fictional_company']=='Tekes Build' and screen['extras']['remove_after_completion']
    screen_primitives=primitives('construction-dust-screen')
    assert any(gltf['materials'][p['material']].get('alphaMode')=='MASK' for p in screen_primitives)
    outer=bounds(['construction-dust-screen']);scaffold=bounds(['construction-scaffold'])
    assert all(outer[k][0]<scaffold[k][0] and outer[k][1]>scaffold[k][1] for k in [0,2]), 'dust screen must wrap the scaffold'
    stories = {}
    for name, floors in [('Terracotta apartments', 4), ('Glass office', 8),
                         ('Limestone hotel', 6), ('Shops and apartments', 4),
                         ('construction-facade', 4)]:
        walls = [p for p in primitives(name) if 'facade_grid' in gltf['materials'][p['material']].get('extras',{})]
        assert walls, name
        all_positions=[]
        for wall in walls:
            pos=values(wall['attributes']['POSITION']);uv=values(wall['attributes']['TEXCOORD_0'])
            grid=gltf['materials'][wall['material']].get('extras',{}).get('facade_grid',[512/24,1024/32])
            all_positions.extend(pos)
            for q in range(0,len(pos),4):
                corners=pos[q:q+4];coords=uv[q:q+4]
                height=max(p[1] for p in corners)-min(p[1] for p in corners)
                rows=(max(p[1] for p in coords)-min(p[1] for p in coords))*grid[1]
                bays=(max(p[0] for p in coords)-min(p[0] for p in coords))*grid[0]
                assert abs(rows-round(rows))<1e-4 and abs(bays-round(bays))<1e-4
                assert abs(height*METRES/rows-3.2)<1e-4, name
        height=max(p[1] for p in all_positions)-min(p[1] for p in all_positions)
        assert abs(height/.8-floors)<1e-4, name
        if name!='construction-facade':
            ground_height=(min(p[1] for p in all_positions)-.17)*METRES
            assert abs(ground_height-3.8)<1e-4, name
        stories[name] = dict(upper_floors=floors, floor_metres=3.2, ground_metres=3.8)

    for name in ['Glass office','Limestone hotel','Shops and apartments']:
        points=[p for prim in primitives(name) if 'facade_grid' in gltf['materials'][prim['material']].get('extras',{}) for p in values(prim['attributes']['POSITION'])]
        roof=max(p[1] for p in points);top=[p for p in points if abs(p[1]-roof)<1e-4]
        assert all(max(p[k] for p in top)-min(p[k] for p in top) < max(p[k] for p in points)-min(p[k] for p in points)-.1 for k in [0,2]), 'real upper-story setback'
    for material in gltf['materials']:
        if 'facade_grid' not in material.get('extras',{}):continue
        texture=gltf['textures'][material['pbrMetallicRoughness']['baseColorTexture']['index']]
        image=gltf['images'][texture['source']];view=gltf['bufferViews'][image['bufferView']]
        assert struct.unpack_from('>II',binary,view.get('byteOffset',0)+16)==(512,512)
    patterns=[]
    for material in gltf['materials']:
        pattern=material.get('extras',{}).get('tekes_interior')
        if material.get('extras',{}).get('construction'):
            assert material['extras']['occupancy_rate']==.5
            assert material['emissiveFactor']==[1,1,1] and material['extras']['tekes_interior']==[4,4,5,.5]
            gate=material['extras']['tekes_interior_activation']
            assert gate[0]==1 and gate[1]>60 and gate[2]>gate[1]
            continue
        if pattern is None:continue
        assert material['extras']['tekes_interior'][3]==material['extras']['occupancy_rate']==.5
        assert len(pattern)==4 and pattern[:2]==[4,4] and pattern[3]==.5
        assert material['emissiveFactor']==[1,1,1] and 'emissiveTexture' in material
        patterns.append(pattern)
    assert len(patterns)==4 and len({json.dumps(p,sort_keys=True) for p in patterns})==4
    ambulance=primitives('car-ambulance')[0]
    ambulance_colors=values(ambulance['attributes']['COLOR_0'])
    red=lambda v:abs(v[0]-.5647115)<.01 and abs(v[1]-.0331048)<.01 and abs(v[2]-.0307134)<.01
    assert sum(red(v) for v in ambulance_colors)>=48, 'red medical cross on both sides'
    entrances={};garages={}
    expected={'Terracotta apartments':'west','Glass office':'north','Limestone hotel':'east','Shops and apartments':'south','construction-facade':'west'}
    for node in gltf['nodes']:
        entry=node.get('extras',{}).get('entrance')
        if entry is None:continue
        assert entry['side']==expected[node['name']]
        nx,_,nz=entry['normal'];px,base,pz=entry['position']
        color=entry['door_color']
        linear=lambda v:v/255/12.92 if v<=10.31475 else ((v/255+.055)/1.055)**2.4
        target=[linear(v) for v in color]
        door=[];walls=[]
        for prim in primitives(node['name']):
            positions=values(prim['attributes']['POSITION'])
            colors_index=prim['attributes']['COLOR_0']
            acc=gltf['accessors'][colors_index];view=gltf['bufferViews'][acc['bufferView']]
            offset=view.get('byteOffset',0)+acc.get('byteOffset',0)
            colors=struct.unpack_from('<'+'f'*acc['count']*4,binary,offset)
            door.extend(p for j,p in enumerate(positions) if all(abs(colors[j*4+k]-target[k])<1e-5 for k in range(3)))
            if 'facade_grid' in gltf['materials'][prim['material']].get('extras',{}):walls.extend(positions)
        assert door
        height=max(p[1] for p in door)-min(p[1] for p in door)
        assert abs(height*METRES-2.24)<1e-4
        plane=entry.get('door_plane_offset',.022)
        assert all(abs((p[0]-px)*nx+(p[2]-pz)*nz-plane)<.021 for p in door), 'door must occupy its recessed street-facing aperture'
        assert abs((max(p[0]*nx+p[2]*nz for p in walls))-(px*nx+pz*nz))<1e-4
        # Ray-probe the actual indexed triangles, not just their metadata: the
        # visible doorway must be behind the outer facade, with no wall sealing it.
        ray_depths=[]
        for prim in primitives(node['name']):
            positions=values(prim['attributes']['POSITION'])
            acc=gltf['accessors'][prim['indices']];view=gltf['bufferViews'][acc['bufferView']]
            code='I' if acc['componentType']==5125 else 'H'
            indices=struct.unpack_from('<'+code*acc['count'],binary,view.get('byteOffset',0)+acc.get('byteOffset',0))
            for j in range(0,len(indices),3):
                points=[positions[k] for k in indices[j:j+3]]
                q=[((p[0]-px)*nz-(p[2]-pz)*nx,p[1]-base) for p in points]
                (ax,ay),(bx,by),(cx,cy)=q
                den=(by-cy)*(ax-cx)+(cx-bx)*(ay-cy)
                if abs(den)<1e-9:continue
                a=((by-cy)*(-cx)+(cx-bx)*(.28-cy))/den
                b=((cy-ay)*(-cx)+(ax-cx)*(.28-cy))/den
                c=1-a-b
                if min(a,b,c)<-1e-5:continue
                depth=sum(weight*((p[0]-px)*nx+(p[2]-pz)*nz) for weight,p in zip([a,b,c],points))
                if -.3<depth<.15:ray_depths.append(depth)
        assert ray_depths and max(ray_depths)<-.05, 'real recessed doorway, no solid wall over the opening'

        offset=recipe['construction']['position'] if node['name']=='construction-facade' else [0,0,0]
        world=[px+offset[0],base+offset[1],pz+offset[2]]
        end=entry['street_connection']
        assert (end[0]-world[0])*nx+(end[2]-world[2])*nz>0
        entrances[node['name']]=entry['side']
        if 'garage' in entry:
            garage=entry['garage'];gx,gy,gz=garage['position'];ex,ey,ez=garage['street_connection']
            assert abs(garage['width']*METRES-3.04)<1e-4 and abs(garage['height']*METRES-2.4)<1e-4
            assert (ex-gx)*nx+(ez-gz)*nz>0 and ey<gy
            assert abs((gx-px)*nz-(gz-pz)*nx)>.55, 'separate pedestrian and vehicle openings'
            garages[node['name']]=garage
    assert len(entrances)==5 and set(garages)=={'Glass office','Limestone hotel'}
    # Inspect exported footprints and the full entry-to-street corridors. A
    # layout edit must not leave a door or driveway aimed into the next building.
    lots={n['name']:n['extras']['footprint'] for n in gltf['nodes'] if 'footprint' in n.get('extras',{})}
    sx,_,sz=recipe['construction']['position']
    lots['construction-facade']=[sx-1.23,sz-1.03,sx+1.23,sz+1.03]
    names=list(lots)
    for i,name in enumerate(names):
        ax,az,bx,bz=lots[name]
        for other in names[i+1:]:
            cx,cz,dx,dz=lots[other]
            assert min(bx,dx)-max(ax,cx)<1e-5 or min(bz,dz)-max(az,cz)<1e-5, ('overlapping buildings',name,other)
    terra,office,hotel,shops=[lots[n] for n in ['Terracotta apartments','Glass office','Limestone hotel','Shops and apartments']]
    assert abs(terra[2]-office[0])<1e-5, 'shared party wall'
    assert abs((hotel[0]-office[2])*METRES-1.0)<1e-5, 'north service alley'
    assert abs((shops[1]-hotel[3])*METRES-1.4)<1e-5, 'east service alley'
    for node in gltf['nodes']:
        entry=node.get('extras',{}).get('entrance')
        if entry is None:continue
        offset=recipe['construction']['position'] if node['name']=='construction-facade' else [0,0,0]
        for portal,half_width in [(entry,.18)]+([(entry['garage'],.42)] if 'garage' in entry else []):
            x,_,z=portal['position'];x+=offset[0];z+=offset[2]
            ex,_,ez=portal['street_connection'];nx,_,nz=entry['normal']
            corridor=[min(x,ex)-(half_width if nz else 0),min(z,ez)-(half_width if nx else 0),max(x,ex)+(half_width if nz else 0),max(z,ez)+(half_width if nx else 0)]
            for other,lot in lots.items():
                if other==node['name']:continue
                assert min(corridor[2],lot[2])-max(corridor[0],lot[0])<1e-5 or min(corridor[3],lot[3])-max(corridor[1],lot[1])<1e-5, ('blocked street connection',node['name'],other)
    alley_route=next(i for i,r in enumerate(recipe['routes']) if r['name']=='Walkable service alleys')
    assert sum(p['route']==alley_route for p in recipe['pedestrians'])==4
    points=recipe['routes'][alley_route]['points']
    for a,b in zip(points,points[1:]+points[:1]):
        for step in range(41):
            x=a[0]+(b[0]-a[0])*step/40;z=a[2]+(b[2]-a[2])*step/40
            for name,(x0,z0,x1,z1) in lots.items():
                assert not (x0-.08<x<x1+.08 and z0-.08<z<z1+.08), ('pedestrian clearance in alley',name,x,z)
    adults=[];children=[]
    assert {a['appearance'] for a in recipe['pedestrians']} == set(range(8))
    assert len({a['speed'] for a in recipe['pedestrians']}) > 20
    for actor in recipe['pedestrians']:
        prefix='person-'+str(actor['appearance'])+'-'
        body=bounds([prefix+'body',prefix+'leg-left',prefix+'leg-right'])
        height=(body[1][1]-body[1][0])*actor['scale']*METRES
        # Heads, torsos and limbs are meshes with real depth, not billboards.
        assert (body[2][1]-body[2][0])*actor['scale']*METRES > .25
        if actor['appearance'] in [2,6]:
            assert 1.0 <= height <= 1.4
            children.append(height)
        else:
            assert 1.55 <= height <= 1.85
            adults.append(height)
    assert max(children) < min(adults) < max(adults) < 2.24 < 3.2 < 3.8
    roofs=[]
    for node in gltf['nodes']:
        if 'roof_units' not in node.get('extras', {}):continue
        units=node['extras']['roof_units'];roofs.append(node['extras'])
        points=[p for prim in primitives(node['name']) if 'facade_grid' in gltf['materials'][prim['material']].get('extras',{}) for p in values(prim['attributes']['POSITION'])]
        top=max(p[1] for p in points);points=[p for p in points if abs(p[1]-top)<1e-4]
        b=[(min(p[k] for p in points),max(p[k] for p in points)) for k in range(3)];cx=(b[0][0]+b[0][1])/2;cz=(b[2][0]+b[2][1])/2
        for i,u in enumerate(units):
            assert b[0][0]+.15 < cx+u['x']-u['width']/2 < cx+u['x']+u['width']/2 < b[0][1]-.15
            assert b[2][0]+.15 < cz+u['z']-u['depth']/2 < cz+u['z']+u['depth']/2 < b[2][1]-.15
            for v in units[i+1:]:
                assert abs(u['x']-v['x'])>(u['width']+v['width'])/2+.119 or abs(u['z']-v['z'])>(u['depth']+v['depth'])/2+.119
    assert len(roofs)==4
    assert len({json.dumps(r,sort_keys=True) for r in roofs})==4
    assert len({tuple(r['awning_color']) for r in roofs})==4
    assert len({tuple(r['door_color']) for r in roofs})==4
    vehicle_names=['car','car-police','car-ambulance','car-fire','car-garbage']
    assert {a['appearance'] for a in recipe['vehicles']} == set(range(5))
    vehicles={};lengths=[];heights=[]
    for actor in recipe['vehicles']:
        name=vehicle_names[actor['appearance']]
        wheel_parts = (['sedan-front','sedan-rear'] if actor['appearance'] <= 1 else
                       ['heavy-front','ambulance-rear'] if actor['appearance'] == 2 else
                       ['heavy-front','heavy-middle','heavy-rear'])
        wheel_nodes = ['road-wheel-'+part for part in wheel_parts]
        # Vehicle proportions include the now independently instanced tires.
        car=bounds([name,*wheel_nodes])
        for wheel in wheel_nodes:
            extent=bounds([wheel])
            assert abs((extent[1][1]-extent[1][0])-(extent[2][1]-extent[2][0])) < .00001, 'circular wheel cross-section'
        length=(car[2][1]-car[2][0])*actor['scale']*METRES
        height=(car[1][1]-car[1][0])*actor['scale']*METRES
        width=(car[0][1]-car[0][0])*actor['scale']*METRES
        assert width < 2.55
        if actor['appearance']==0:
            assert 4.0 <= length <= 4.7 and 1.3 <= height <= 1.7
            lengths.append(length);heights.append(height)
        elif actor['appearance']==1:
            assert 4.0 <= length <= 4.7 and 1.3 <= height <= 1.9
        else:
            assert 5.0 <= length <= 7.0 and 2.4 <= height <= 3.5
        vehicles[name]=dict(length_metres=length,height_metres=height,width_metres=width)
    activities={a['activity'] for a in recipe['pedestrians']}
    assert activities=={'walking','strolling','listening','jogging'}
    joggers=[a['speed'] for a in recipe['pedestrians'] if a['activity']=='jogging']
    strollers=[a['speed'] for a in recipe['pedestrians'] if a['activity']=='strolling']
    assert min(joggers)>max(strollers)
    tree_names = ['tree-broadleaf', 'tree-fanleaf', 'tree-slender']
    tree_heights = [bounds([tree_names[t['species']]])[1][1] * t['scale'] * METRES for t in recipe['trees']]
    assert all(2.8 <= height <= 6.2 for height in tree_heights)
    for mesh in gltf['meshes']:
        for prim in mesh['primitives']:
            assert all(abs(sum(v*v for v in n) - 1.) < .001
                       for n in values(prim['attributes']['NORMAL'])), 'unit normals'
    return dict(scene_unit_metres=METRES, buildings=stories,
                entrances=entrances,garage_entrances=garages,
                window_interiors=[dict(grid=p[:2],seed=p[2],occupancy=p[3]) for p in patterns],
                footprints=lots,party_wall=True,service_alley_metres=[1.0,1.4],
                adult_height_metres=[min(adults), max(adults)],
                child_height_metres=[min(children), max(children)],
                roof_unit_counts=[len(r["roof_units"]) for r in roofs],
                vehicle_dimensions=vehicles,
                activities=sorted(activities),
                pedestrian_speed_metres_per_second=[min(a["speed"] for a in recipe["pedestrians"])*METRES,max(a["speed"] for a in recipe["pedestrians"])*METRES],
                car_length_metres=[min(lengths), max(lengths)],
                car_height_metres=[min(heights), max(heights)],
                tree_height_metres=[min(tree_heights), max(tree_heights)])


if __name__ == '__main__':
    print(json.dumps(check(), indent=2))
