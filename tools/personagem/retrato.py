# WestPlay: the picture of a person on its card in the app's Personagem page (ui/public/characters/<prefix>.png,
# 360x480, transparent). docs/personagem/INCLUIR_PERSONAGEM.md, step 7.
#   blender -b <x_animada.blend> --python retrato.py -- <out.png> parado 60 <height in m>
import bpy, sys, math
a=sys.argv[sys.argv.index('--')+1:]
out, action, frame, height = a[0], a[1], int(a[2]), float(a[3])
for o in list(bpy.data.objects):
    if o.type in ('CAMERA','LIGHT'): bpy.data.objects.remove(o)
arm=[o for o in bpy.data.objects if o.type=='ARMATURE'][0]
arm.animation_data.action=bpy.data.actions[action]
sc=bpy.context.scene; sc.frame_set(frame)
sc.render.engine='BLENDER_EEVEE'; sc.render.film_transparent=True
sc.view_settings.view_transform='Standard'
w=bpy.data.worlds.new("w"); sc.world=w; w.color=(0.35,0.35,0.38)
key=bpy.data.objects.new("key",bpy.data.lights.new("key",'SUN')); key.data.energy=3.2; key.rotation_euler=(math.radians(55),0,math.radians(-35)); sc.collection.objects.link(key)
rim=bpy.data.objects.new("rim",bpy.data.lights.new("rim",'SUN')); rim.data.energy=1.5; rim.rotation_euler=(math.radians(60),0,math.radians(150)); sc.collection.objects.link(rim)
cam=bpy.data.objects.new("cam",bpy.data.cameras.new("cam")); sc.collection.objects.link(cam); sc.camera=cam
cam.data.lens=50; d=height*2.05; ang=math.radians(-22)
cam.location=(math.sin(ang)*d, -math.cos(ang)*d, height*0.55)
cam.rotation_euler=(math.radians(90),0,ang)
sc.render.resolution_x=360; sc.render.resolution_y=480; sc.render.resolution_percentage=100
sc.render.image_settings.file_format='PNG'; sc.render.image_settings.color_mode='RGBA'; sc.render.image_settings.compression=90
sc.render.filepath=out; bpy.ops.render.render(write_still=True)
print("DONE", out)
