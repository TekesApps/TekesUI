"""Original animated-district asset library and independent placement recipe.
Pillow is an offline dependency only. Runtime loads GLB + JSON, no generator.
"""
from pathlib import Path
import math,json,random,copy
from block_geometry import Asset,Geometry
from PIL import Image,ImageDraw
from city_architecture import facade_atlas,segments,relief,ground_relief
from city_shopfronts import ground_storey,entrance,sign_image,masonry_image
from city_construction import add_dust_screen
from city_transit import add_transit
from city_clouds import add_clouds
from city_road_wheels import add_road_wheels

# Shared authoring dimensions: one scene unit represents four metres.
METRES_PER_UNIT=4.0
FLOOR=3.2/METRES_PER_UNIT
GROUND_FLOOR=3.8/METRES_PER_UNIT
GROUND=.17
# Authoring inputs per building. Empty rooms remain empty in both lighting modes.
BUILDING_OCCUPANCY={name:.50 for name in ['Terracotta apartments','Glass office','Limestone hotel','Shops and apartments']}

def driveway(g,plain,entry):
    x,y,z=entry['garage']['position'];nx,_,nz=entry['normal'];tx,tz=nz,-nx
    target={'north':(x,-4.65),'east':(6.05,z),'south':(x,4.65),'west':(-6.05,z)}[entry['side']]
    ex,ez=target;half=.42
    near=[(x-tx*half,y,z-tz*half),(x+tx*half,y,z+tz*half)]
    far=[(ex-tx*half,.108,ez-tz*half),(ex+tx*half,.108,ez+tz*half)]
    top=[near[0],far[0],far[1],near[1]]
    if nx*nx+nz*nz!=1:raise ValueError('axis-aligned frontage required')
    # Both orientations use a top-facing winding.
    ux=[top[1][k]-top[0][k] for k in range(3)];vx=[top[2][k]-top[0][k] for k in range(3)]
    if ux[2]*vx[0]-ux[0]*vx[2]<0:top.reverse()
    g.quad(top,plain,(106,118,116))
    for j in range(4):
        a,b=top[j],top[(j+1)%4]
        g.quad([(a[0],.05,a[2]),(b[0],.05,b[2]),b,a],plain,(133,141,132))
    entry['garage']['street_connection']=[ex,.108,ez]

def approach(g,plain,entry):
    x,_,z=entry['position'];side=entry['side']
    target={'north':(x,-4.48),'east':(5.82,z),'south':(x,4.48),'west':(-5.82,z)}[side]
    tx,tz=target
    # Solid paving joins the entrance to its sidewalk, with no floating slab.
    g.box((x+tx)/2,.11,(z+tz)/2,.36 if x==tx else abs(tx-x),.12,
          .36 if z==tz else abs(tz-z),plain,(192,184,162))
    return [tx,GROUND,tz]

def build():
    a=Asset();plain=a.material('Vertex colors',(255,255,255));stone=a.material('Warm stone',(228,223,205))
    facades=[]
    fixture_lights=[]
    def fixture(position,direction,color,radius,outer_cos,dimming=None,construction=False):
        fixture_lights.append(dict(position=position,direction=direction,color=color,radius=radius,outer_cos=outer_cos,dimming=dimming,construction=construction))

    masonry=a.material('Original ground-floor ashlar',(255,255,255),a.image(masonry_image()))
    lamp=a.material('Warm street lamps',(246,226,175))
    a.materials[lamp]['emissiveFactor']=[1.,.72,.30]
    def fixture_material(name,dimming):
        material=copy.deepcopy(a.materials[lamp]);material['name']=name
        if dimming:material['extras']=dict(tekes_dimming=[2,*dimming])
        a.materials.append(material);return len(a.materials)-1
    door_lamp=fixture_material('Warm entrance wall lamps',[210,300,.35])
    canopy_lamp=fixture_material('Warm canopy downlights',[150,240,0.0])

    headlights=a.material('Night vehicle headlights',(255,250,230));a.materials[headlights]['emissiveFactor']=[1.,.92,.72]
    taillights=a.material('Night vehicle taillights',(200,35,28));a.materials[taillights]['emissiveFactor']=[1.,.025,.015]
    # Independent named objects rather than one city-wide fused mesh.
    g=Geometry(a);g.box(0,-.24,0,19,.48,18,plain,(139,148,139));g.box(0,.02,0,18.5,.06,17.4,plain,(173,172,157))
    # Loop roads around a compact block, with two spatially separated lanes.
    for z in [-5.65,5.65]:g.box(0,.075,z,16.5,.06,2.0,plain,(68,76,79))
    for x in [-7.05,7.05]:g.box(x,.075,0,2.0,.06,11.3,plain,(68,76,79))
    for z in [-5.65,5.65]:
        for x in range(-6,7):g.box(x,.108,z,.48,.003,.035,plain,(218,203,149))
    for x in [-7.05,7.05]:
        for z in range(-4,5):g.box(x,.108,z,.035,.003,.48,plain,(218,203,149))
    for x in [-5.82,5.82]:g.box(x,.13,0,.30,.06,9.8,plain,(202,200,181))
    for z in [-4.48,4.48]:g.box(0,.13,z,11.9,.06,.30,plain,(202,200,181))
    # Zebra crossings and curb ramps; first demo uses non-crossing actor routes.
    for x in [-4.8,4.8]:
        for z in [-5.65,5.65]:
            for i in range(7):g.box(x,.111,z-.78+i*.26,.65,.004,.14,plain,(226,225,204))
    # Waterfront promenade and retaining wall.
    g.box(8.3,.08,0,.85,.16,17.7,plain,(188,180,155));g.box(12.05,.08,0,.55,.16,17.7,plain,(188,180,155));g.box(12.45,-.15,0,.24,.65,18.0,plain,(119,128,120))
    for z in range(-8,9):g.box(12.24,.385,z,.025,.30,.025,plain,(79,105,105))
    for y in [.36,.52]:g.box(12.24,y,0,.025,.025,17,plain,(111,130,126))
    g.box(14.4,-.5,0,4.0,.35,18,plain,(34,91,111));g.finish('streets-and-quay')
    # North row: apartments and office share a party wall. Two 1.0/1.4m
    # service alleys join the north and east street walls around an open courtyard.
    recipes=[(-3.3,-2.65,3.2,2.6,4,1,'Terracotta apartments','west'),(-.125,-2.65,3.15,2.6,8,0,'Glass office','north'),(3.2,-2.65,3.0,2.6,6,2,'Limestone hotel','east'),(3.2,1.25,3.0,4.5,4,2,'Shops and apartments','south')]
    palettes=[((54,79,91),(48,97,106)),((103,78,59),(145,75,52)),((75,85,67),(95,115,78)),((83,71,87),(105,81,112))]
    random.Random(731025).shuffle(palettes)
    approaches=[]
    garage_entries=[]
    for building,(x,z,w,d,upper_floors,style,name,side) in enumerate(recipes):
        occupancy=BUILDING_OCCUPANCY[name]
        wall_facade=a.material(name+' original windows',(255,255,255),a.image(facade_atlas(style,building+1)))
        a.materials[wall_facade].update(emissiveFactor=[1,1,1],emissiveTexture=dict(index=a.image(facade_atlas(style,building+1,True))),extras=dict(tekes_interior=[4,4,building+1,occupancy],occupancy_rate=occupancy,facade_grid=[4,4]))
        facades.append(wall_facade)
        detail=random.Random('city-roof:'+name)
        door_color,awning_color=palettes[building]
        blocked_faces=[(1,0)] if building==0 else [(-1,0)] if building==1 else []
        h=GROUND_FLOOR+upper_floors*FLOOR;roof=GROUND+h
        g=Geometry(a)
        has_garage=name in ['Glass office','Limestone hotel']
        ground_storey(g,plain,x,z,w,d,GROUND,side,[(145,161,156),(174,139,114),(198,191,171)][style],has_garage,wall_material=masonry)
        ground_relief(g,plain,x,z,w,d,GROUND,side,blocked_faces=blocked_faces)
        parts=segments(w,d,upper_floors,style,name);floor_base=GROUND+GROUND_FLOOR
        for pw,pd,levels in parts:
            g.box(x,floor_base+levels*FLOOR/2,z,pw,levels*FLOOR,pd,stone,facade=wall_facade,facade_floor_height=FLOOR,facade_bay_width=.50,facade_grid=(4,4))
            relief(g,plain,x,z,pw,pd,floor_base,levels,FLOOR,style,balconies=style==1,blocked_faces=blocked_faces if pw==w else ())
            floor_base+=levels*FLOOR
        rw,rd,_=parts[-1]
        g.box(x,roof+.04,z,rw+.12,.08,rd+.12,plain,(176,178,166))
        for dx in [-rw/2,rw/2]:g.box(x+dx,roof+.17,z,.06,.26,rd,plain,(181,184,174))
        for dz in [-rd/2,rd/2]:g.box(x,roof+.17,z+dz,rw,.26,.06,plain,(181,184,174))
        # Seeded roof layout: complete service units, with access gaps and parapet clearance.
        placed=[]
        for unit in range(detail.randint(1,4)):
            uw,ud=detail.uniform(.32,.45),detail.uniform(.42,.58)
            if detail.choice([False,True]):uw,ud=ud,uw
            for attempt in range(80):
                ux=detail.uniform(-rw/2+uw/2+.18,rw/2-uw/2-.18)
                uz=detail.uniform(-rd/2+ud/2+.18,rd/2-ud/2-.18)
                if all(abs(ux-px)>(uw+pw)/2+.12 or abs(uz-pz)>(ud+pd)/2+.12 for px,pz,pw,pd in placed):break
            else:continue
            placed.append((ux,uz,uw,ud));uh=detail.uniform(.16,.25)
            shade=detail.choice([(139,155,151),(166,174,163),(151,159,163)])
            g.box(x+ux,roof+.08+uh/2,z+uz,uw,uh,ud,plain,shade)
            g.cylinder(x+ux,roof+.08+uh+.012,z+uz,min(uw,ud)*.32,.024,plain,(61,75,78))
        brand,kind=[('CORNER CAFE','cafe'),('RIVER WORKSPACE','office'),('TERRACE HOTEL','hotel'),('RIVER MARKET','market')][building]
        sign=a.material(brand,(255,255,255),a.image(sign_image(brand,awning_color)))
        a.materials[sign].update(emissiveFactor=[.3,.3,.3],emissiveTexture=a.materials[sign]['pbrMetallicRoughness']['baseColorTexture'])
        entry=entrance(g,plain,x,z,w,d,GROUND,side,door_color,awning_color,has_garage,sign_material=sign,kind=kind)
        # Fixtures follow this entrance's own outward normal, including side doors.
        px,_,pz=entry['position'];nx,_,nz=entry['normal'];tx,tz=nz,-nx
        for u in [-.34,.34]:
            lx,lz=px+tx*u+nx*.055,pz+tz*u+nz*.055;ly=GROUND+.61
            g.box(lx,ly,lz,.075 if nz else .05,.12,.05 if nz else .075,plain,(55,65,61))
            g.box(lx+nx*.03,ly,lz+nz*.03,.055 if nz else .025,.075,.025 if nz else .055,door_lamp,(255,255,255))
            fixture([lx+nx*.05,ly-.01,lz+nz*.05],[nx*.25,-1.,nz*.25],[2.2,1.35,.55],1.6,.32,[210,300,.35])
        for u in [-.22,.22]:
            lx,lz=px+tx*u+nx*.24,pz+tz*u+nz*.24;ly=GROUND+.78
            g.box(lx,ly,lz,.075,.018,.075,canopy_lamp,(255,255,255))
            fixture([lx,ly-.02,lz],[0.,-1.,0.],[1.4,.95,.48],1.35,.40,[150,240,0.0])
        if has_garage:garage_entries.append(entry)
        approaches.append(entry)
        g.finish(name)
        a.nodes[-1]['extras']=dict(footprint=[x-w/2,z-d/2,x+w/2,z+d/2],party_wall_normals=blocked_faces,roof_footprint=[x,z,rw,rd],roof_units=[dict(x=px,z=pz,width=pw,depth=pd) for px,pz,pw,pd in placed],door_color=door_color,awning_color=awning_color,entrance=entry)
    g=Geometry(a);g.box(0,.10,1.25,3.2,.12,3.0,plain,(91,128,80));g.box(0,.17,1.25,.45,.02,3.0,plain,(192,181,152));g.box(0,.17,1.25,3.2,.02,.38,plain,(192,181,152))
    for x,z in [(-1.18,.5),(1.18,2.05),(-1.2,2.1)]:
        for y in [.30,.46]:g.box(x,y,z,.48,.035,.16 if y==.30 else .035,plain,(138,110,74))
        for dx in [-.19,.19]:g.box(x+dx,.23,z,.025,.14,.10,plain,(68,88,82))
    # Pond basin, low masonry rim and fountain pedestal.
    g.box(0,.25,-.15,1.55,.32,.85,plain,(173,174,157));g.box(0,.43,-.15,1.35,.06,.65,plain,(57,122,131))
    for x in [-.78,.78]:g.box(x,.46,-.15,.10,.15,.95,plain,(194,189,163))
    g.finish('pocket-park')
    # Street lamps and small bollards; all static geometry batched by object.
    g=Geometry(a)
    for x,z in [(-5.5,-3.6),(5.5,-3.6),(-5.5,3.6),(5.5,3.6),(8.25,-5),(8.25,5)]:
        dimming=[180,270,0.0 if x>8 else .55]
        bulb=fixture_material('Street light '+str((x,z)),dimming)
        g.cylinder(x,1.25,z,.033,2.2,plain,(62,87,85));g.box(x,2.35,z,.25,.10,.30,bulb,(255,255,255))
        fixture([x,2.32,z],[0.,-1.,0.],[2.8,1.85,.85],4.8,.42,dimming)
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
    water('water',12.6,16.3,-8.9,8.9,-.13,.16);water('pond',-.65,.65,-.45,.15,.47,.06)
    # A construction lot is independent of the reusable frame and crane prototypes.
    g=Geometry(a);g.box(-3.6,.135,.25,2.9,.07,2.7,plain,(167,155,126));g.finish('construction-lot')
    g=Geometry(a)
    for x in [-5.04,-2.16]:g.box(x,.42,.25,.035,.5,2.7,plain,(97,125,126))
    for z in [-1.08,1.58]:g.box(-3.6,.42,z,2.9,.5,.035,plain,(110,135,132))
    for i in range(4):g.box(-4.4+i*.28,.38,1.10,.20,.28,.65,plain,(166,134,96))
    g.finish('construction-hoarding')
    g=Geometry(a)
    for level in range(6):
        y=.04 if level==0 else .04+GROUND_FLOOR+(level-1)*FLOOR
        g.box(0,y,0,2.5,.085,2.1,plain,(186,185,168))
        if level<5:
            for x in [-1.1,0,1.1]:
                for z in [-.9,.9]:g.box(x,y+(GROUND_FLOOR if level==0 else FLOOR)/2,z,.11,GROUND_FLOOR if level==0 else FLOOR,.11,plain,(163,161,145))
            # Different stages of enclosure on the lower floors.
            if level<2:g.box(0,y+.39,-1.,2.45,.69,.08,plain,(190,154,120))
    g.finish('construction-frame')
    # The finished facade joins the same shared window system. A GPU clock gate
    # keeps every room unoccupied/unlit until the site has been handed over.
    site_facade=copy.deepcopy(a.materials[facades[0]])
    site_facade['name']='Construction to occupied apartments windows'
    site_facade['extras']=dict(tekes_interior=[4,4,5,.5],occupancy_rate=.5,construction=True,facade_grid=[4,4])
    a.materials.append(site_facade);site_facade_id=len(a.materials)-1
    g=Geometry(a)
    for x,z in [(-5.1,1.4),(-2.1,-1.0)]:
        g.cylinder(x,1.35,z,.028,2.4,plain,(84,91,76))
        g.box(x,2.7,z,.30,.17,.12,plain,(83,91,83))
        g.box(x,2.69,z+.065,.25,.10,.014,lamp,(255,255,255))
        fixture([x,2.69,z+.065],[.8 if x<-4 else -.8,-.9,-.5 if x<-4 else .6],[4.8,4.6,3.7],5.2,.5,construction=True)
    g.finish('construction-worklights')
    # Enclosure follows the structural frame, so the finished building stays intact.
    g=Geometry(a);ground_storey(g,plain,0,0,2.46,2.06,.04,'west',(174,139,114),False,wall_material=masonry);g.box(0,.04+GROUND_FLOOR+2*FLOOR,0,2.46,4*FLOOR,2.06,plain,(245,239,226),facade=site_facade_id,facade_floor_height=FLOOR,facade_bay_width=.50,facade_grid=(4,4))
    relief(g,plain,0,0,2.46,2.06,.04+GROUND_FLOOR,4,FLOOR,1)
    ground_relief(g,plain,0,0,2.46,2.06,.04,'west')
    site_sign=a.material('New apartments lobby',(255,255,255),a.image(sign_image('GARDEN APARTMENTS',(104,94,71))))
    site_entry=entrance(g,plain,0,0,2.46,2.06,.04,'west',(66,92,93),(104,94,71),sign_material=site_sign)
    g.finish('construction-facade');a.nodes[-1]['extras']=dict(entrance=site_entry)
    approaches.append(dict(side='west',position=[-3.6-1.23,GROUND,.25]))
    g=Geometry(a)
    for entry in approaches:entry['street_connection']=approach(g,plain,entry)
    g.finish('entrance-approaches')
    g=Geometry(a)
    for entry in garage_entries:driveway(g,plain,entry)
    g.finish('garage-driveways')
    site_entry['street_connection']=[-5.82,GROUND,.25]
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
    add_dust_screen(a)
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
    # Original streamlined electric sedan: tapered nose, sloped glass and panoramic roof.
    g=Geometry(a);paint=(224,230,229);glass=(198,220,228)
    car_glass=a.material('Car transparent glazing',(255,255,255))
    a.materials[car_glass].update(alphaMode='BLEND')
    a.materials[car_glass]['pbrMetallicRoughness']['baseColorFactor'][3]=.20
    def normal(a,b):
        n=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
        length=math.sqrt(sum(v*v for v in n));return [v/length for v in n]
    def delta(a,b):return [a[k]-b[k] for k in range(3)]
    def ring(y,w,front,rear):
        r=.14;points=[]
        for cx,cz,start in [(-w+r,rear+r,1.5*math.pi),(-w+r,front-r,math.pi),(w-r,front-r,.5*math.pi),(w-r,rear+r,0)]:
            for k in range(9):
                angle=start-k*math.pi/16;points.append((cx+r*math.cos(angle),y,cz+r*math.sin(angle)))
        return points
    rings=[ring(.19,.34,.77,-.78),ring(.34,.39,.83,-.83),ring(.46,.36,.73,-.75)]
    count=len(rings[0]);ns=[]
    for j,row in enumerate(rings):
        ns.append([normal(delta(row[(i+1)%count],row[(i-1)%count]),delta(rings[min(j+1,2)][i],rings[max(j-1,0)][i])) for i in range(count)])
    for j in range(2):
        for i in range(count):
            k=(i+1)%count
            g.quad([rings[j][i],rings[j][k],rings[j+1][k],rings[j+1][i]],plain,paint,normals=[ns[j][i],ns[j][k],ns[j+1][k],ns[j+1][i]])
    for i in range(0,count,2):g.quad([(0,.46,0),rings[2][i],rings[2][(i+1)%count],rings[2][(i+2)%count]],plain,paint)
    # Curved windscreen, roof and rear glass share smooth analytic mesh normals.
    profile=[(-.59,.46,.32),(-.47,.62,.29),(-.32,.73,.275),(-.07,.78,.27),(.15,.75,.275),(.31,.64,.295),(.47,.46,.32)]
    glass_rows=[[( -w*math.cos(k*math.pi/12),h+.025*math.sin(k*math.pi/12),z) for k in range(13)] for z,h,w in profile]
    gn=[]
    for j,row in enumerate(glass_rows):
        gn.append([normal(delta(glass_rows[min(j+1,6)][i],glass_rows[max(j-1,0)][i]),delta(row[min(i+1,12)],row[max(i-1,0)])) for i in range(13)])
    for j in range(6):
        for i in range(12):g.quad([glass_rows[j][i],glass_rows[j+1][i],glass_rows[j+1][i+1],glass_rows[j][i+1]],car_glass,glass,normals=[gn[j][i],gn[j+1][i],gn[j+1][i+1],gn[j][i+1]])
        for side in [-1,1]:
            z0,h0,w0=profile[j];z1,h1,w1=profile[j+1]
            pts=[(side*w0,.46,z0),(side*w1,.46,z1),(side*w1,h1,z1),(side*w0,h0,z0)]
            if j in [0,5]:
                # End points meet the body; triangles are encoded as indexed quads.
                if j==0:pts=[pts[0],pts[1],pts[2],((pts[0][0]+pts[2][0])/2,(pts[0][1]+pts[2][1])/2,(pts[0][2]+pts[2][2])/2)]
                else:pts=[pts[0],pts[1],pts[3],((pts[3][0]+pts[0][0])/2,(pts[3][1]+pts[0][1])/2,(pts[3][2]+pts[0][2])/2)]
            if side<0:pts.reverse()
            g.quad(pts,car_glass,glass,normals=[(side,0,0)]*4)
    for side in [-1,1]:
        g.beam((side*.32,.46,.47),(side*.27,.77,.12),.018,plain,paint)
        g.beam((side*.32,.46,-.59),(side*.27,.74,-.35),.018,plain,paint)
        for z in [-.28,.24]:g.box(side*.365,.445,z,.012,.018,.10,plain,(86,103,108))
        g.box(side*.25,.365,.791,.19,.028,.025,headlights,(255,255,255))
        g.box(side*.25,.36,-.799,.19,.028,.025,taillights,(255,255,255))
    # Simple original cockpit, visible through the newly transparent glazing.
    g.box(0,.48,.25,.49,.045,.18,plain,(50,58,64))
    g.box(0,.52,.25,.16,.09,.012,plain,(25,34,41))
    g.box(0,.525,.243,.13,.055,.008,plain,(128,166,175))
    # Ring steering wheel, angled toward the driver rather than a solid disc.
    for j in range(24):
        a0=j*math.tau/24;a1=(j+1)*math.tau/24
        g.beam((.12+.072*math.cos(a0),.55+.057*math.sin(a0),.17),(.12+.072*math.cos(a1),.55+.057*math.sin(a1),.17),.008,plain,(37,43,48))
    for x in [-.12,.12]:
        g.box(x,.48,-.11,.19,.05,.27,plain,(73,80,83))
        g.box(x,.56,-.22,.19,.16,.045,plain,(73,80,83))
    # Standard passenger-car proportions: lower cabin and 1.8–2.1 m width.
    for positions,normals,_,_,_ in g.groups.values():
        for k in range(0,len(positions),3):
            positions[k]*=.90;positions[k+1]*=.78
            n=[normals[k]/.90,normals[k+1]/.78,normals[k+2]];length=math.sqrt(sum(v*v for v in n))
            normals[k:k+3]=[v/length for v in n]
    g.finish('car')
    sedan=copy.deepcopy(g.groups)
    g=Geometry(a);g.groups=copy.deepcopy(sedan)
    for side in [-1,1]:g.box(side*.328,.28,-.06,.016,.16,.72,plain,(42,61,85))
    g.box(0,.63,-.08,.42,.04,.13,plain,(42,49,54))
    g.box(-.11,.67,-.08,.20,.05,.12,plain,(52,104,209));g.box(.11,.67,-.08,.20,.05,.12,plain,(206,53,49))
    g.finish('car-police')
    for kind,name in [(2,'car-ambulance'),(3,'car-fire'),(4,'car-garbage')]:
        g=Geometry(a);long=kind!=2;rear=-1.36 if long else -1.10
        color=(239,239,220) if kind==2 else (185,53,43) if kind==3 else (69,124,91)
        g.box(0,.26,(rear+1.05)/2,.88,.20,1.05-rear,plain,(56,66,70))
        # Separate forward cab and box/compactor rear body.
        g.box(0,.58,.64,.85,.62,.76,plain,(237,238,220) if kind==4 else color)
        g.box(0,.74,1.025,.72,.28,.018,plain,(52,83,95))
        for side in [-1,1]:
            g.box(side*.432,.74,.65,.018,.28,.54,plain,(55,83,95))
            g.box(side*.27,.41,1.036,.18,.06,.025,headlights,(255,255,255))
        if kind==2:
            g.box(0,.68,-.37,.89,.88,1.40,plain,color)
            for side in [-1,1]:
                g.box(side*.451,.49,-.36,.018,.12,1.30,plain,(164,62,54))
                g.box(side*.452,.84,-.36,.018,.29,.29,plain,(237,239,226))
                g.box(side*.465,.84,-.36,.018,.19,.055,plain,(198,51,49))
                g.box(side*.465,.84,-.36,.018,.055,.19,plain,(198,51,49))
            g.box(-.21,1.15,-.04,.24,.06,.13,plain,(61,116,212));g.box(.21,1.15,-.04,.24,.06,.13,plain,(204,61,53))
        elif kind==3:
            g.box(0,.64,-.49,.92,.72,1.67,plain,color)
            for side in [-1,1]:
                g.box(side*.466,.56,-.49,.018,.42,1.51,plain,(158,164,155))
                for z in [-1.10,-.65,-.20]:g.box(side*.48,.56,z,.018,.35,.025,plain,(91,111,114))
            for x in [-.27,.27]:g.beam((x,1.05,-1.26),(x,1.05,.13),.035,plain,(192,196,182))
            for j in range(9):g.beam((-.27,1.05,-1.20+j*.15),(.27,1.05,-1.20+j*.15),.025,plain,(203,205,188))
            g.box(-.20,.94,.65,.23,.06,.15,plain,(61,111,205));g.box(.20,.94,.65,.23,.06,.15,plain,(200,57,52))
        else:
            g.box(0,.78,-.40,.93,.99,1.53,plain,color)
            g.box(0,.71,-1.22,.94,.75,.30,plain,(48,76,64))
            for side in [-1,1]:
                g.box(side*.474,.82,-.40,.012,.09,1.35,plain,(202,207,149))
                g.box(side*.474,.48,-.40,.012,.08,1.35,plain,(202,207,149))
            g.box(0,.94,.65,.36,.06,.13,plain,(220,158,52))
        for side in [-1,1]:g.box(side*.30,.42,rear-.015,.16,.07,.025,taillights,(255,255,255))
        g.finish(name)
    add_road_wheels(a,plain)
    # Volumetric people: independent clothing/skin/hair colors, not a whole-person tint.
    looks=[
        ((216,170,132),(48,36,30),(57,102,145),(49,63,79),'short',False),
        ((145,93,65),(37,27,24),(170,74,71),(57,61,81),'long',True),
        ((235,192,158),(173,115,57),(220,166,60),(50,96,127),'cap',False),
        ((100,65,48),(65,55,52),(82,126,93),(70,67,62),'short',False),
        ((222,178,146),(192,192,181),(139,118,157),(83,80,78),'short',False),
        ((181,127,91),(211,210,194),(150,99,71),(75,79,89),'bob',True),
        ((157,107,79),(51,32,24),(176,100,152),(69,89,113),'long',True),
        ((236,194,159),(157,94,48),(64,133,136),(70,75,89),'bob',False),
    ]
    for variant,(skin,hair,shirt,trousers,style,skirt) in enumerate(looks):
        prefix='person-'+str(variant)+'-';g=Geometry(a)
        g.ellipsoid(0,.70,0,.14,.205,.115,plain,shirt,sides=12,rings=8)
        g.ellipsoid(0,.51,0,.13,.08,.11,plain,shirt,sides=12,rings=6)
        for side in [-1,1]:g.ellipsoid(side*.125,.845,0,.065,.075,.075,plain,shirt,sides=12,rings=6)
        g.cylinder(0,.93,0,.038,.08,plain,skin)
        g.ellipsoid(0,1.025,.005,.075,.09,.078,plain,skin,sides=12,rings=8)
        g.ellipsoid(0,1.09,-.012,.077,.04,.080,plain,hair,sides=12,rings=6)
        g.ellipsoid(0,1.02,.084,.014,.020,.018,plain,skin,sides=8,rings=4)
        if style in ['long','bob']:
            g.ellipsoid(0,1.015 if style=='bob' else .97,-.054,.084,.085 if style=='bob' else .13,.045,plain,hair,sides=12,rings=6)
        if style=='cap':
            g.ellipsoid(0,1.103,0,.086,.043,.085,plain,(71,119,154),sides=12,rings=6)
            g.box(0,1.105,.075,.15,.014,.11,plain,(71,119,154))
        if skirt:g.ellipsoid(0,.48,0,.17,.20,.125,plain,shirt,sides=12,rings=8)
        g.finish(prefix+'body')
        for side in [-1,1]:
            g=Geometry(a);g.cylinder(side*.075,.30,0,.047,.40,plain,trousers)
            g.ellipsoid(side*.075,.30,0,.048,.205,.055,plain,trousers,sides=10,rings=6)
            g.ellipsoid(side*.075,.065,.035,.06,.045,.115,plain,(49,52,56),sides=10,rings=6)
            g.finish(prefix+('leg-left' if side<0 else 'leg-right'))
            g=Geometry(a);g.cylinder(side*.18,.69,0,.041,.32,plain,shirt)
            g.ellipsoid(side*.18,.85,0,.045,.05,.048,plain,shirt,sides=10,rings=6)
            g.ellipsoid(side*.18,.49,.015,.039,.055,.045,plain,skin,sides=10,rings=6)
            g.finish(prefix+('arm-left' if side<0 else 'arm-right'))
            g=Geometry(a);g.beam((side*.18,.86,0),(side*.18,.64,0),.075,plain,shirt)
            g.beam((side*.18,.64,0),(side*.18,.69,.17),.066,plain,shirt)
            g.ellipsoid(side*.18,.69,.17,.039,.045,.05,plain,skin,sides=10,rings=6)
            g.finish(prefix+('arm-left-run' if side<0 else 'arm-right-run'))
        g=Geometry(a)
        for side in [-1,1]:g.ellipsoid(side*.084,1.045,0,.022,.041,.036,plain,(43,52,64),sides=10,rings=6)
        for j in range(8):
            a0=math.pi*j/8;a1=math.pi*(j+1)/8
            g.beam((.09*math.cos(a0),1.075+.09*math.sin(a0),0),(.09*math.cos(a1),1.075+.09*math.sin(a1),0),.014,plain,(44,53,63))
        g.finish(prefix+'headphones')
    # A small motorboat faces +Z; hull/deck remain separate indexed surfaces.
    g=Geometry(a)
    top=[(-.28,.17,-.78),(.28,.17,-.78),(.30,.17,.52),(.10,.15,1.02),(-.10,.15,1.02),(-.30,.17,.52)]
    bottom=[(x*.65,-.15,z*.90) for x,y,z in top]
    for i in range(6):g.quad([bottom[i],bottom[(i+1)%6],top[(i+1)%6],top[i]],plain,(156,67,54))
    for ids in [[0,1,2,5],[5,2,3,4]]:g.quad([top[i] for i in ids],plain,(231,226,204))
    g.box(0,.37,-.22,.43,.40,.61,plain,(58,103,123));g.box(0,.59,-.22,.50,.055,.69,plain,(235,235,221))
    for x in [-.22,.22]:
        for z in [-.50,.06]:g.box(x,.37,z,.026,.41,.026,plain,(224,224,207))
    g.box(0,.26,-.60,.40,.12,.17,plain,(179,169,137));g.box(0,.02,-.83,.15,.28,.13,plain,(57,70,76))
    g.cylinder(0,.83,-.40,.012,.44,plain,(218,220,201));g.box(.07,.94,-.40,.15,.10,.012,plain,(202,120,74));g.finish('boat')
    g=Geometry(a)
    for side in [-1,1]:
        for i in range(16):
            d0=.92+i*.105;d1=d0+.085
            x0=side*(.14+.13*d0);x1=side*(.14+.13*d1);width=.020*(1-i/22)
            g.quad([(x0-width,.012,-d0),(x0+width,.012,-d0),(x1+width,.012,-d1),(x1-width,.012,-d1)],plain,(163,198,193))
    g.finish('boat-wake')
    # Rounded closed routes eliminate abrupt right-angle turns.
    def loop(x0,x1,z0,z1,r,y,reverse=False):
        points=[]
        for cx,cz,start in [(x1-r,z1-r,0),(x0+r,z1-r,90),(x0+r,z0+r,180),(x1-r,z0+r,270)]:
            for k in range(9):
                t=math.radians(start+k*90/8);points.append([round(cx+r*math.cos(t),5),y,round(cz+r*math.sin(t),5)])
        return list(reversed(points)) if reverse else points
    routes=[dict(name='Outer traffic lane',points=loop(-7.55,7.55,-6.15,6.15,1.0,.13)),dict(name='Inner opposing lane',points=loop(-6.55,6.55,-5.15,5.15,.85,.13,True)),dict(name='Sidewalk clockwise',points=loop(-5.45,5.45,-4.05,4.05,.6,.17)),dict(name='Sidewalk counterclockwise',points=loop(-5.14,5.14,-3.82,3.82,.55,.17,True)),dict(name='Park paths',points=loop(-1.3,1.3,.20,2.6,.2,.18))]
    routes.append(dict(name='River navigation',points=loop(13.7,15.2,-6.4,6.4,.65,-.13)))
    routes.append(dict(name='Walkable service alleys',points=loop(1.575,5.45,-4.12,-1.175,.065,GROUND)))
    boats=[dict(route=5,distance=4.5,speed=.85,color=[1,1,1],scale=.9,phase=0)]
    rng=random.Random(241020);vehicles=[];pedestrians=[]
    for route in [0,1]:
        for index in range(5):
            appearance=([0,1,0,2,0] if route==0 else [3,0,4,0,0])[index]
            color=rng.choice([[.96,.97,1],[.55,.60,.65],[.27,.34,.43],[1,.38,.32],[.34,.36,.40]]) if appearance==0 else [1,1,1]
            vehicles.append(dict(route=route,distance=index*10.1,speed=1.35 if route==0 else 1.15,speed_variation=rng.uniform(.22,.45),color=color,scale=rng.uniform(.60,.63) if appearance>=2 else rng.uniform(.60,.68),appearance=appearance,phase=rng.uniform(0,math.tau)))
    for route,count in [(2,12),(3,10),(4,6)]:
        for index in range(count):
            appearance=len(pedestrians)%len(looks)
            child=appearance in [2,6];elder=appearance in [4,5]
            scale=rng.uniform(.25,.30) if child else rng.uniform(.36,.40)
            activity=['walking','strolling','listening','jogging','walking'][len(pedestrians)%5]
            if elder:activity='strolling'
            speed=rng.uniform(.60,.88) if activity=='jogging' else rng.uniform(.18,.25) if activity=='strolling' else rng.uniform(.25,.35) if activity=='listening' else rng.uniform(.28,.43)
            pedestrians.append(dict(route=route,distance=index*(3.2 if route<4 else 1.6),speed=speed,speed_variation=rng.uniform(.015,.04),lateral_offset=rng.uniform(-.025,.025),color=[1,1,1],scale=scale,appearance=appearance,activity=activity,phase=rng.uniform(0,math.tau)))
    # Use existing appearance prototypes and GPU route instances. Keep lateral
    # drift off in the narrow passage; each person still has independent speed.
    for index,appearance in enumerate([0,2,4,6]):
        pedestrians.append(dict(route=6,distance=index*3.2,speed=.21+index*.025,speed_variation=.01,lateral_offset=0,color=[1,1,1],scale=.28 if appearance in [2,6] else .38,appearance=appearance,activity='strolling',phase=index*1.7))
    trees=[]
    for x,z in [(-5.8,-2),(-5.8,.1),(-5.8,2.3),(5.8,-2),(5.8,.1),(5.8,2.3),(-3.2,4.42),(-.8,4.42),(1.5,4.42),(3.8,4.42),(-3.2,-4.42),(3.8,-4.42),(-.95,1.0),(.8,1.7),(.7,.6),(8.2,-3),(8.2,0),(8.2,3)]:trees.append(dict(position=[x,GROUND,z],species=len(trees)%3,scale=rng.uniform(.38,.58),phase=rng.uniform(0,math.tau)))
    trains=add_transit(a,plain,headlights,taillights,routes)
    add_clouds(a)
    station_demand=dict(occupied_residents=sum(levels*8*BUILDING_OCCUPANCY[name] for _,_,_,_,levels,_,name,_ in recipes if name in ['Terracotta apartments','Shops and apartments']),occupied_jobs=sum(levels*10*BUILDING_OCCUPANCY[name] for _,_,_,_,levels,_,name,_ in recipes if name not in ['Terracotta apartments','Shops and apartments']),rail_share=.6)
    config=dict(station_demand=station_demand,trains=trains,asset='city-block.glb',lights=fixture_lights,construction=dict(position=[-3.6,GROUND,.25],crane_position=[-4.6,GROUND,-.55],facade_delay_floors=.75,scaffold_hold_seconds=6,scaffold_removal_seconds_per_floor=2.5,floor_height=FLOOR,levels=5.3,seconds_per_floor=8,phase=1.4),routes=routes,boats=boats,vehicles=vehicles,pedestrians=pedestrians,trees=trees,wind_amplitude=.035,wave_amplitude=.065)
    c=config['construction']
    handover=(math.ceil(c['levels'])-c['phase'])*c['seconds_per_floor']+c['scaffold_hold_seconds']+c['facade_delay_floors']*c['seconds_per_floor']+math.ceil(c['levels'])*c['scaffold_removal_seconds_per_floor']+4
    site_facade['extras']['tekes_interior_activation']=[1,handover,handover+2,0]
    return a.finish(),config
if __name__=='__main__':
    data,config=build();folder=Path(__file__).parent;(folder/'city-block.glb').write_bytes(data);(folder/'city-block.json').write_text(json.dumps(config,indent=2)+'\n');print(len(data),'bytes;',len(config['vehicles']),'cars,',len(config['pedestrians']),'pedestrians,',len(config['trees']),'trees')
