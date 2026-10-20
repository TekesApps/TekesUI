"""Original animated-district asset library and independent placement recipe.
Pillow is an offline dependency only. Runtime loads GLB + JSON, no generator.
"""
from pathlib import Path
import math,json,random
from block_geometry import Asset,Geometry,facade
from PIL import Image,ImageDraw

def build():
    a=Asset();plain=a.material('Vertex colors',(255,255,255));stone=a.material('Warm stone',(228,223,205))
    facades=[a.material(name,(255,255,255),a.image(facade(i))) for i,name in enumerate(['Glass curtain wall','Terracotta apartments','Limestone facade'])]
    # Independent named objects rather than one city-wide fused mesh.
    g=Geometry(a);g.box(0,-.24,0,19,.48,18,plain,(139,148,139));g.box(0,.02,0,18.5,.06,17.4,plain,(173,172,157))
    # Loop roads around a compact block, with two spatially separated lanes.
    for z in [-5.65,5.65]:g.box(0,.075,z,16.5,.06,2.0,plain,(68,76,79))
    for x in [-7.05,7.05]:g.box(x,.075,0,2.0,.06,11.3,plain,(68,76,79))
    for z in [-5.65,5.65]:
        for x in range(-6,7):g.box(x,.108,z,.48,.003,.035,plain,(218,203,149))
    for x in [-7.05,7.05]:
        for z in range(-4,5):g.box(x,.108,z,.035,.003,.48,plain,(218,203,149))
    for x in [-5.82,5.82]:g.box(x,.16,0,.30,.20,9.8,plain,(202,200,181))
    for z in [-4.48,4.48]:g.box(0,.16,z,11.9,.20,.30,plain,(202,200,181))
    # Zebra crossings and curb ramps; first demo uses non-crossing actor routes.
    for x in [-4.8,4.8]:
        for z in [-5.65,5.65]:
            for i in range(7):g.box(x,.111,z-.78+i*.26,.65,.004,.14,plain,(226,225,204))
    # Waterfront promenade and retaining wall.
    g.box(8.3,.12,0,.85,.24,17.7,plain,(188,180,155));g.box(8.85,-.15,0,.24,.65,18.0,plain,(119,128,120))
    for z in range(-8,9):g.box(8.64,.53,z,.025,.75,.025,plain,(79,105,105))
    for y in [.45,.78]:g.box(8.64,y,0,.025,.025,17,plain,(111,130,126))
    g.box(10.8,-.4,0,4.0,.35,18,plain,(34,91,111));g.finish('streets-and-quay')
    recipes=[(-3.55,-2.35,2.7,2.5,4.3,1,'Terracotta apartments'),(.0,-2.65,2.8,2.15,7.6,0,'Glass office'),(3.65,-2.4,2.3,2.5,5.5,2,'Limestone hotel'),(3.6,2.15,2.7,2.2,3.8,2,'Shops and apartments')]
    for x,z,w,d,h,style,name in recipes:
        g=Geometry(a);g.box(x,.22+h/2,z,w,h,d,stone,facade=facades[style]);g.box(x,.24+h,z,w+.12,.09,d+.12,plain,(176,178,166))
        # Rooftop setbacks, parapets, AC fans and service equipment.
        for dx in [-w/2,w/2]:g.box(x+dx,h+.40,z,.06,.28,d,plain,(181,184,174))
        for dz in [-d/2,d/2]:g.box(x,h+.40,z+dz,w,.28,.06,plain,(181,184,174))
        for dx in [-.48,.40]:
            g.box(x+dx,h+.43,z,.42,.26,.55,plain,(143,158,157));g.cylinder(x+dx,h+.58,z,.15,.025,plain,(66,83,88))
        # Ground-floor shopfronts, recessed doors, awnings and stoops.
        for dx in [-w*.28,w*.28]:g.box(x+dx,.70,z+d/2+.018,w*.32,.75,.035,plain,(40,82,90))
        g.box(x,.65,z+d/2+.022,.38,1.05,.04,plain,(66,92,93));g.box(x,1.35,z+d/2+.18,w*.92,.08,.4,plain,(45,111,100) if style==1 else (129,72,58))
        g.box(x,.17,z+d/2+.30,.70,.22,.4,plain,(155,159,147));g.finish(name)
    g=Geometry(a);g.box(0,.15,1.25,3.7,.16,3.0,plain,(91,128,80));g.box(0,.24,1.25,.45,.035,3.0,plain,(192,181,152));g.box(0,.24,1.25,3.7,.035,.38,plain,(192,181,152))
    for x,z in [(-1.18,.5),(1.18,2.05),(-1.2,2.1)]:
        for y in [.48,.86]:g.box(x,y,z,.7,.055,.20 if y==.48 else .045,plain,(138,110,74))
        for dx in [-.26,.26]:g.box(x+dx,.35,z,.035,.25,.12,plain,(68,88,82))
    # Pond basin, low masonry rim and fountain pedestal.
    g.box(0,.25,-.15,1.55,.32,.85,plain,(173,174,157));g.box(0,.43,-.15,1.35,.06,.65,plain,(57,122,131))
    for x in [-.78,.78]:g.box(x,.46,-.15,.10,.15,.95,plain,(194,189,163))
    g.finish('pocket-park')
    # Street lamps and small bollards; all static geometry batched by object.
    g=Geometry(a)
    for x,z in [(-5.5,-3.6),(5.5,-3.6),(-5.5,3.6),(5.5,3.6),(8.25,-5),(8.25,5)]:
        g.cylinder(x,1.25,z,.033,2.2,plain,(62,87,85));g.box(x,2.35,z,.25,.10,.30,plain,(235,226,184))
    g.finish('street-furniture')
    # Dynamic water meshes: indexed grids, smooth authored normals.
    def water(name,x0,x1,z0,z1,y,step):
        g=Geometry(a);nx=math.ceil((x1-x0)/step);nz=math.ceil((z1-z0)/step)
        # Share all grid vertices, avoiding triangle expansion.
        ps=[];ns=[];uv=[];colors=[];indices=[]
        for iz in range(nz+1):
            for ix in range(nx+1):ps.extend([x0+(x1-x0)*ix/nx,y,z0+(z1-z0)*iz/nz]);ns.extend([0,1,0]);uv.extend([ix/nx,iz/nz]);colors.extend([.035,.23,.30,1.])
        for iz in range(nz):
            for ix in range(nx):
                i=iz*(nx+1)+ix;indices.extend([i,i+nx+1,i+1,i+1,i+nx+1,i+nx+2])
        g.groups[plain]=[ps,ns,uv,colors,indices];g.finish(name)
    water('water',9.0,12.7,-8.9,8.9,-.13,.16);water('pond',-.65,.65,-.45,.15,.47,.06)
    # A construction lot is independent of the reusable frame and crane prototypes.
    g=Geometry(a);g.box(-3.6,.18,2.2,2.9,.18,2.7,plain,(167,155,126))
    for x in [-5.04,-2.16]:g.box(x,.60,2.2,.035,.8,2.7,plain,(97,125,126))
    for z in [.87,3.53]:g.box(-3.6,.60,z,2.9,.8,.035,plain,(110,135,132))
    for i in range(4):g.box(-4.4+i*.28,.38,3.05,.20,.28,.65,plain,(166,134,96))
    g.finish('construction-lot')
    g=Geometry(a)
    for level in range(6):
        y=.16+level*.78
        g.box(0,y,0,2.5,.085,2.1,plain,(186,185,168))
        if level<5:
            for x in [-1.1,0,1.1]:
                for z in [-.9,.9]:g.box(x,y+.39,z,.11,.78,.11,plain,(163,161,145))
            # Different stages of enclosure on the lower floors.
            if level<2:g.box(0,y+.39,-1.,2.45,.69,.08,plain,(190,154,120))
    g.finish('construction-frame')
    # Enclosure follows the structural frame, so the finished building stays intact.
    g=Geometry(a);g.box(0,2.11,0,2.46,3.90,2.06,plain,(245,239,226),facade=facades[1]);g.finish('construction-facade')
    # Independent outside scaffold: fixed-size posts, diagonal braces, rails and boards.
    g=Geometry(a);steel=(184,180,147);rail=(222,170,72)
    for level in range(5):
        y=.16+level*.78
        for z in [-1.17,1.17]:
            for x in [-1.34,-.45,.45,1.34]:g.beam((x,y,z),(x,y+.78,z),.032,plain,steel)
            g.box(0,y+.08,z,2.82,.045,.17,plain,(155,126,84))
            for h in [.39,.70]:g.beam((-1.34,y+h,z),(1.34,y+h,z),.025,plain,rail)
            for j in range(3):
                x=-1.34+j*.893;g.beam((x,y+.1,z),(x+.893,y+.75,z),.022,plain,steel)
        for x in [-1.34,1.34]:
            g.beam((x,y,0),(x,y+.78,0),.032,plain,steel)
            g.box(x,y+.08,0,.17,.045,2.34,plain,(155,126,84))
            for h in [.39,.70]:g.beam((x,y+h,-1.17),(x,y+h,1.17),.025,plain,rail)
            g.beam((x,y+.1,-1.17),(x,y+.75,0),.022,plain,steel);g.beam((x,y+.1,0),(x,y+.75,1.17),.022,plain,steel)
    g.finish('construction-scaffold')
    yellow=(223,174,66);g=Geometry(a)
    for x in [-.16,.16]:
        for z in [-.16,.16]:g.beam((x,0,z),(x,6.2,z),.045,plain,yellow)
    for level in range(12):
        y=level*.5
        for z in [-.16,.16]:g.beam((-.16,y,z),(.16,y+.5,z),.026,plain,yellow)
        for x in [-.16,.16]:g.beam((x,y,-.16),(x,y+.5,.16),.026,plain,yellow)
    g.box(0,.14,0,.75,.28,.75,plain,(134,139,129));g.box(.32,5.80,0,.38,.40,.38,plain,(102,144,150));g.finish('crane-tower')
    g=Geometry(a)
    for z in [-.16,.16]:
        for y in [6.,6.30]:g.beam((-1.0,y,z),(3.2,y,z),.035,plain,yellow)
        for i in range(14):
            x=-1+i*.3;g.beam((x,6.,z),(x+.3,6.3,z),.022,plain,yellow)
    g.box(-.85,5.85,0,.50,.32,.65,plain,(101,113,113))
    g.beam((2.2,6.,0),(2.2,4.70,0),.015,plain,(66,76,76));g.box(2.2,4.62,0,.11,.16,.11,plain,yellow);g.finish('crane-boom')
    # Shared rooted tree prototype. Vertex colors keep foliage/trunk one draw.
    leaf_rng=random.Random(43021)
    for species,name in enumerate(['tree-broadleaf','tree-fanleaf','tree-slender']):
        leaf_color=[(100,149,70),(149,171,75),(68,128,80)][species]
        im=Image.new('RGBA',(128,128),leaf_color+(0,));draw=ImageDraw.Draw(im)
        if species==1:shape=[(64,120),(12,48),(25,14),(64,4),(104,14),(116,48)]
        elif species==2:shape=[(64,124),(43,78),(42,36),(64,3),(86,36),(85,78)]
        else:shape=[(64,124),(22,89),(9,48),(31,12),(64,3),(97,12),(119,48),(106,89)]
        draw.polygon(shape,fill=leaf_color+(255,));draw.line([(64,119),(64,12)],fill=(172,186,111,255),width=2)
        for y in [35,55,75,95]:
            draw.line([(64,y+10),(31,y-12)],fill=(126,164,89,255),width=1);draw.line([(64,y+10),(96,y-12)],fill=(126,164,89,255),width=1)
        leaf=a.material('Leaf veins '+str(species),(255,255,255),a.image(im),masked=True)
        g=Geometry(a);g.cylinder(0,.60,0,.045,1.2,plain,(112,94,68))
        rx,ry,rz=[(.53,.63,.48),(.62,.48,.52),(.31,.88,.32)][species]
        for j in range(7):
            angle=j*math.tau/7;end=(rx*.75*math.cos(angle),1.5+ry*.3*math.sin(j),rz*.75*math.sin(angle));g.beam((0,.65,0),end,.026,plain,(115,97,69))
        # Individually oriented leaf sprays, not solid ellipsoid crowns.
        for j in range(280):
            theta=leaf_rng.uniform(0,math.tau);v=leaf_rng.uniform(-1,1);r=leaf_rng.uniform(.35,1)**.4
            x=rx*r*math.sqrt(1-v*v)*math.cos(theta);z=rz*r*math.sqrt(1-v*v)*math.sin(theta);y=1.48+ry*r*v
            angle=leaf_rng.uniform(0,math.tau);size=leaf_rng.uniform(.09,.17);u=(math.cos(angle)*size,0,math.sin(angle)*size);w=(-math.sin(angle)*size*.4,size,math.cos(angle)*size*.4)
            points=[(x+aa*u[0]+bb*w[0],y+bb*w[1],z+aa*u[2]+bb*w[2]) for aa,bb in [(-1,-1),(1,-1),(1,1),(-1,1)]]
            shade=leaf_rng.randint(205,255);g.quad(points,leaf,(shade,shade,shade))
        g.finish(name)
    # Vehicle prototype facing +Z. Opaque stylized glazing avoids sorting instances.
    g=Geometry(a);g.box(0,.31,0,.76,.28,1.64,plain,(221,225,215));g.box(0,.57,-.03,.65,.31,.84,plain,(52,91,107));g.box(0,.75,-.08,.65,.06,.75,plain,(226,229,217));g.box(0,.46,.62,.74,.10,.36,plain,(227,230,220));g.box(0,.42,-.68,.74,.08,.23,plain,(211,216,207))
    for x in [-.39,.39]:
        for z in [-.49,.51]:g.cylinder(x,.21,z,.17,.075,plain,(35,42,44),axis='x')
    for x in [-.25,.25]:g.box(x,.33,.83,.16,.08,.02,plain,(245,228,172));g.box(x,.33,-.83,.16,.07,.02,plain,(159,53,48))
    g.finish('car')
    # Articulated pedestrian prototypes. Rigid GPU swing about explicit joints.
    g=Geometry(a);g.box(0,.70,0,.25,.39,.16,plain,(190,201,199));g.ellipsoid(0,1.00,0,.105,.13,.10,plain,(209,171,134),sides=12,rings=6);g.ellipsoid(0,1.075,-.012,.105,.065,.10,plain,(80,67,56),sides=12,rings=4);g.finish('person-body')
    for side in [-1,1]:
        g=Geometry(a);g.box(side*.075,.30,0,.095,.40,.12,plain,(63,80,91));g.box(side*.075,.065,.035,.11,.08,.19,plain,(45,51,52));g.finish('person-leg-left' if side<0 else 'person-leg-right')
        g=Geometry(a);g.box(side*.18,.69,0,.075,.32,.10,plain,(185,198,197));g.ellipsoid(side*.18,.49,.015,.043,.065,.045,plain,(211,172,135),sides=8,rings=4);g.finish('person-arm-left' if side<0 else 'person-arm-right')
    # Rounded closed routes eliminate abrupt right-angle turns.
    def loop(x0,x1,z0,z1,r,y,reverse=False):
        points=[]
        for cx,cz,start in [(x1-r,z1-r,0),(x0+r,z1-r,90),(x0+r,z0+r,180),(x1-r,z0+r,270)]:
            for k in range(9):
                t=math.radians(start+k*90/8);points.append([round(cx+r*math.cos(t),5),y,round(cz+r*math.sin(t),5)])
        return list(reversed(points)) if reverse else points
    routes=[dict(name='Outer traffic lane',points=loop(-7.55,7.55,-6.15,6.15,1.0,.13)),dict(name='Inner opposing lane',points=loop(-6.55,6.55,-5.15,5.15,.85,.13,True)),dict(name='Sidewalk clockwise',points=loop(-5.45,5.45,-4.05,4.05,.6,.26)),dict(name='Sidewalk counterclockwise',points=loop(-5.14,5.14,-3.82,3.82,.55,.26,True)),dict(name='Park paths',points=loop(-1.7,1.7,.20,2.6,.2,.27))]
    rng=random.Random(241020);vehicles=[];pedestrians=[]
    for route in [0,1]:
        for index in range(5):vehicles.append(dict(route=route,distance=index*10.1,speed=1.35 if route==0 else 1.15,speed_variation=rng.uniform(.22,.45),color=rng.choice([[1,.83,.45],[.65,.85,1],[1,.6,.52],[.79,1,.86],[.95,.96,1]]),scale=rng.uniform(.62,.72),phase=rng.uniform(0,math.tau)))
    for route,count in [(2,12),(3,10),(4,6)]:
        for index in range(count):pedestrians.append(dict(route=route,distance=index*(3.2 if route<4 else 1.6),speed=rng.uniform(.40,.80) if route<4 else rng.uniform(.30,.55),lateral_offset=rng.uniform(-.09,.09),color=rng.choice([[.9,.65,.55],[.58,.79,1],[.65,.96,.75],[1,.89,.65],[.84,.71,1]]),scale=rng.uniform(.53,.65),phase=rng.uniform(0,math.tau)))
    trees=[]
    for x,z in [(-5.8,-2),(-5.8,.1),(-5.8,2.3),(5.8,-2),(5.8,.1),(5.8,2.3),(-3.2,4.42),(-.8,4.42),(1.5,4.42),(3.8,4.42),(-3.2,-4.42),(3.8,-4.42),(-.95,1.0),(.8,1.7),(.7,.6),(8.2,-3),(8.2,0),(8.2,3)]:trees.append(dict(position=[x,.25,z],species=len(trees)%3,scale=rng.uniform(.48,.74),phase=rng.uniform(0,math.tau)))
    config=dict(asset='city-block.glb',construction=dict(position=[-3.6,.24,2.2],crane_position=[-4.6,.24,1.4],facade_delay_floors=.75,scaffold_hold_seconds=6,scaffold_removal_seconds_per_floor=2.5,floor_height=.78,levels=5.3,seconds_per_floor=8,phase=1.4),routes=routes,vehicles=vehicles,pedestrians=pedestrians,trees=trees,wind_amplitude=.035,wave_amplitude=.032)
    return a.finish(),config
if __name__=='__main__':
    data,config=build();folder=Path(__file__).parent;(folder/'city-block.glb').write_bytes(data);(folder/'city-block.json').write_text(json.dumps(config,indent=2)+'\n');print(len(data),'bytes;',len(config['vehicles']),'cars,',len(config['pedestrians']),'pedestrians,',len(config['trees']),'trees')
