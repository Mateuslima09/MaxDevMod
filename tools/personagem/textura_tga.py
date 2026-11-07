# WestPlay: a person's colour texture as the TGA SCS's converter takes (it refuses PNG), its colours as they are
# (no view transform). docs/personagem/INCLUIR_PERSONAGEM.md, step 5.
#   blender -b --factory-startup --python textura_tga.py -- <in.png> <out.tga>
import bpy, sys
A = sys.argv[sys.argv.index('--') + 1:]
im = bpy.data.images.load(A[0])
sc = bpy.context.scene
sc.view_settings.view_transform = 'Standard'; sc.view_settings.look = 'None'
sc.view_settings.exposure = 0; sc.view_settings.gamma = 1
sc.render.image_settings.file_format = 'TARGA'; sc.render.image_settings.color_mode = 'RGB'
im.save_render(A[1], scene=sc)
print("TGA", A[1], tuple(im.size))
