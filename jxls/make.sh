#!/bin/sh
# Makes the pictures of this directory again with the program itself:
#   sh jxls/make.sh [path of symbolic-math] [pattern of names, as 'plot4d_*']
# Every file is lossless and packed as hard as the encoder can (effort = 10).
# A file that is there is left as it is.
B=${1:-symbolic-math}
ONLY=${2:-*}
cd "$(dirname "$0")" || exit 1

# name, drawing, what is chosen besides
one() {
  case "$1" in $ONLY) ;; *) return ;; esac
  [ -f "$1.jxl" ] && return
  "$B" "save_jxl(\"$1.jxl\", $2$3)" | tail -1
}
# a shape of four dimensions: 512 x 512 pixels, 512 frames, 512 colours
shape() { one "$1" "$2" ""; }
# one picture
picture() { one "$1" "$2" ", width = 1920, height = 1080, effort = 10"; }
# 512 frames of 512 x 512 pixels: the eye goes around once, or a walk grows
turning() { one "$1" "$2" ", width = 512, height = 512, frames = 512, fps = 24, effort = 10"; }

shape plot4d_sin 'plot4d(sin(z))'
shape plot4d_gamma_body 'plot4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat'
shape plot4d_three_poles_body 'plot4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma'
shape plot4d_tan 'plot4d(tan(z), z, 3), range_real = [-3, 3], range_imag = [-3, 3], palette = turbo'
shape plot4d_zeta_body 'plot4d(zeta(z), z, -6 - 12i, 6 + 12i), fill = below, palette = inferno, range_real = [-3, 3], range_imag = [-3, 3]'
shape plot4d_waves_body 'plot4d(sin(z)*cos(z), z, pi), fill = below, palette = twilight, range_real = [-2, 2], range_imag = [-2, 2]'
shape plot4d_ribbons 'plot4d(z^5 - z, z, 1.5), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2'
shape plot4d_rings 'plot4d(z*sin(z), z, 6), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2'
shape plot4d_circles 'plot4d(e^z, z, -2 - pi*i, 2 + pi*i), arg_real = frame, arg_imag = color, val_real = width, val_imag = height, normalize, thickness = 3'
shape plot4d_bows 'plot4d(z^2, z, 2), arg_real = frame, arg_imag = color, val_real = width, val_imag = height, normalize, thickness = 2, palette = spectrum'
shape plot4d_moebius 'plot4d((z - 1)/(z + 1), z, 3), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, range_real = [-3, 5], range_imag = [-4, 4], thickness = 2'
shape plot4d_cube_bands 'plot4d(z^3 - 1, z, 1.5), fill = below, palette = cube, normalize'
shape plot4d_turned_sheet 'plot4d(sin(z), z, pi), rotate(35, 25, 10), thickness = 2, palette = turbo'
shape plot4d_curves 'plot4d(sin(z)*cos(z)), arg_imag = frame, val_real = height, normalize, thickness = 3'
shape plot4d_gray9 'plot4d(z + 1/z), gray, bits = 9, range_real = [-4, 4], range_imag = [-4, 4]'
shape plot4d_cube3 'plot4d(exp(z), z, 3), palette = cube, bits = 3, fill = below'

picture view4d_sin 'view4d(sin(z))'
picture view4d_gamma 'view4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat'
picture view4d_three_poles 'view4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma, range_real = [-3, 3], perspective = 3'
picture view4d_tan 'view4d(tan(z), z, 3), range_real = [-3, 3], range_imag = [-3, 3], palette = turbo'
picture view4d_zeta 'view4d(zeta(z), z, -6 - 12i, 6 + 12i), fill = below, palette = inferno, range_real = [-3, 3], range_imag = [-3, 3], voxels = [256, 512, 256]'
picture view4d_carpet 'view4d(z + 1/z), range_real = [-4, 4], range_imag = [-4, 4], palette = turbo, azimuth = 30, elevation = 60'
picture view4d_waves 'view4d(sin(z)*cos(z), z, pi), palette = twilight, range_real = [-2, 2], range_imag = [-2, 2]'
picture view4d_ribbons 'view4d(z^5 - z, z, 1.5), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, thickness = 2'
picture view4d_rings 'view4d(z*sin(z), z, 6), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2'
picture view4d_root 'view4d(sqrt(z)), val_real = color, val_imag = frame, background = white'
picture view4d_hang 'view4d(z^4 - 1, z, 1.3), fill = above, palette = plasma'
picture view4d_two_poles_stereo 'view4d(1/(z^2 + 1), z, 2), range_real = [-2, 2], range_imag = [-2, 2], stereo = 5'
picture view4d_erf_turned 'view4d(erf(z), z, 2), fill = below, rotate(25, 20, 0), palette = coolwarm, range_real = [-3, 3], range_imag = [-3, 3]'
picture view4d_exp 'view4d(e^z, z, -2 - pi*i, 2 + pi*i), title = "the exponential function"'
turning view4d_around_sin 'view4d(sin(z))'
turning view4d_around_gamma 'view4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat'
turning view4d_around_poles 'view4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma, range_real = [-3, 3]'

picture walk_pi 'walk(pi, 10, 200000)'
picture walk_e_base4 'walk(e, 4, 300000), palette = fire'
picture walk_sqrt2_base6 'walk(sqrt(2), 6, 200000), palette = viridis'
picture primewalk 'primewalk(200000)'
picture primemodwalk 'primemodwalk(300000, 12)'
picture ulamspiral 'ulamspiral(500000)'
picture sacksspiral 'sacksspiral(200000)'
picture walk3d_near 'walk3d(e + pi*i, 12, 100000), perspective = 3, fog = 0.8'
picture walk3d_stereo 'walk3d(sqrt(2) + sqrt(3)*i, 12, 60000), stereo'
picture walk3d_depth 'walk3d(e + pi*i, 12, 100000), depth, azimuth = 300, elevation = 35'
picture primewalk3d 'primewalk3d(100000), axes'
picture primemodwalk3d 'primemodwalk3d(100000)'
picture primes3d_xyz 'primes3d_xyz(20000, 4)'
turning walk_pi_grows 'walk(pi, 10, 100000), grow'
turning walk3d_around 'walk3d(e + pi*i, 12, 50000)'
one walk3d_slices 'walk3d(e + pi*i, 12, 100000)' ', slices = 512, width = 512, height = 512, fps = 24, effort = 10'

picture plot3d_ripples 'plot3d(sin(sqrt(x^2 + y^2))/sqrt(x^2 + y^2), x, -12, 12, y, -12, 12), palette = turbo, perspective = 3'
picture plot3d_saddle 'plot3d(x^2 - y^2, x, -2, 2, y, -2, 2), palette = turbo'
picture surf_peaks 'surf(peaks(80)), palette = turbo, azimuth = 50, elevation = 30'
picture surfc_peaks 'surfc(peaks(60)), palette = jet'
picture waterfall_peaks 'waterfall(peaks(40))'
picture hyperboloid 'fimplicit3(x^2 + y^2 - z^2 = 1, x, -2, 2, y, -2, 2, z, -2, 2)'
picture contour3_cells 'contour3(sin(x)*cos(y), x, -pi, pi, y, -pi, pi)'
picture bar3_magic 'bar3(magic(6))'
picture helix 'scatter3(cos((0:400)/10), sin((0:400)/10), (0:400)/40)'
picture knot 'parametricplot3d([(2 + cos(3t))*cos(2t), (2 + cos(3t))*sin(2t), sin(3t)], t, 0, 2pi), thickness = 3, perspective = 4'
turning plot3d_around 'plot3d(sin(sqrt(x^2 + y^2))/sqrt(x^2 + y^2), x, -12, 12, y, -12, 12), palette = turbo'
turning knot_around 'parametricplot3d([(2 + cos(3t))*cos(2t), (2 + cos(3t))*sin(2t), sin(3t)], t, 0, 2pi), thickness = 3'

picture complexplot_cubic 'complexplot(z^3 - 1, z, -2.4 - 1.35i, 2.4 + 1.35i)'
picture complexplot_gamma 'complexplot(gamma(z), z, -6.4 - 3.6i, 6.4 + 3.6i)'
picture complexplot_zeta 'complexplot(zeta(z), z, -16 - 9i, 16 + 9i)'
picture complexplot_essential 'complexplot(sin(1/z), z, -0.8 - 0.45i, 0.8 + 0.45i)'
picture complexplot_blaschke 'complexplot((z^2 - 1)/(z^2 + 1), z, -3.2 - 1.8i, 3.2 + 1.8i)'
picture densityplot_waves 'densityplot(sin(x)*cos(y) + sin(x*y/3), x, -8, 8, y, -4.5, 4.5), palette = twilight'
picture contour_cells 'contour(sin(x)*cos(y), x, -2pi, 2pi, y, -pi, pi)'
picture implicitplot_heart 'implicitplot((x^2 + y^2 - 1)^3 = x^2*y^3, x, -2.4, 2.4, y, -1.2, 1.5)'
picture streamline_spiral 'streamline([-y, x + y/2], x, -2, 2, y, -1.2, 1.2)'
picture plot_four_curves 'plot([sin(x), cos(x), sin(2x)/2, x*cos(3x)/6], x, -2pi, 2pi), grid, title = "four curves"'
picture parametricplot_lissajous 'parametricplot([sin(3t), sin(4t)], t, 0, 2pi)'
picture polarplot_rose 'polarplot(cos(7t/4), t, 0, 8pi)'
picture bodeplot 'bodeplot(tf([1], [1, 0.4, 1]))'
picture nyquistplot 'nyquistplot(tf([4], [1, 3, 3, 1]))'
picture rlocusplot 'rlocusplot(tf([1, 2], [1, 3, 5, 0]))'
picture graphplot_petersen 'graphplot(petersengraph())'
