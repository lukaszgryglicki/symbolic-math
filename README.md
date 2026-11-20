# symbolic-math

A computer algebra system for the command line, written in Rust. It simplifies expressions,
solves equations, inequalities and systems of them, differentiates, integrates, takes limits,
sums series, solves differential, functional and Diophantine equations and recurrences, works
with matrices, tensors, lists, whole numbers, special functions, integral transforms,
statistics, geometry, graphs and quantities with units, applies a function a fractional or
complex number of times, writes numbers in any base, draws plots with characters, walks along
the digits of numbers and along the primes, saves its drawings as PNG and JPEG XL pictures,
and evaluates everything to as many digits as asked for. Most of what MATLAB computes is here
too, under MATLAB's names: arrays, statistics and tests, matrices taken apart, polynomials,
filters, texts and patterns, dates, optimisation, charts of data, records and tables; see
[What MATLAB has](#what-matlab-has).

Numbers are complex by default. Results are exact wherever an exact form is found, and each
exact constant is followed by its numeric value with at least 32 significant digits (more when a
number of the input has more).

```
$ symbolic-math '4*(x^2)+2x+2=0'
x = -1/4 - i*sqrt(7)/4
  ~ -0.25 - 0.66143782776614764762540393840982i
x = -1/4 + i*sqrt(7)/4
  ~ -0.25 + 0.66143782776614764762540393840982i
$ symbolic-math 'sin(x)=0' '10*x-(2+3)*x' 'i^2' '2^^x=65536'
x = k*pi, k is an integer
5x
-1
x = 4
```

Tetration (`^^`, `tet`) and its inverses (`slog`, `ssrt`) are calculated by the external `tet`
executable of [claude-tetration](https://github.com/lukaszgryglicki/claude-tetration) when the
answer is not a plain tower of powers; see [Tetration](#tetration).

## Build and install

Rust (edition 2024, tested with 1.97 and 1.98), a C and a C++ compiler, `m4`, `cmake` and
`libclang` are needed: [rug](https://crates.io/crates/rug) builds GMP, MPFR and MPC from
source, and [jxl-sys](https://crates.io/crates/jxl-sys) builds libjxl (the JPEG XL encoder)
with cmake and makes its bindings with libclang. The other dependencies are plain Rust:
[png](https://crates.io/crates/png) and [font8x8](https://crates.io/crates/font8x8) (the
letters in pictures). FreeBSD, Linux and macOS are supported; the same input gives the same
PNG file, byte for byte, on all of them.

```sh
cargo build --release --locked
./target/release/symbolic-math 'integrate(x*e^x, x)'
```

The Makefile builds the same executable and installs it:

```sh
make release    # target/release/symbolic-math
make static     # a static executable (on macOS: one that needs the system libraries only;
                # on Linux the static C++ library libstdc++.a must be installed)
make install    # copies it to ./symbolic-math, ./symbolic-math.static and /data/scripts/symbolic-math
make install INSTALL_DIR="$HOME/bin"
make test       # unit and integration tests
make lint       # cargo fmt --check and cargo clippy -D warnings
make clean      # removes target/, not those copies
```

## Use

```
symbolic-math [OPTIONS] [EXPRESSION]...
```

Each argument is one input; quote it for the shell. An input holds one or more statements
separated by `;`. Without arguments the program reads inputs from the standard input, one a
line (`quit` ends it, lines that start with `#` are skipped). All inputs of one run share their
definitions, assumptions and the last result (`ans` or `%`).

| Option | Meaning |
| --- | --- |
| `-d N`, `--digits N` | significant digits of numeric values (default 32, or more when a number of the input has more digits) |
| `-r`, `--real` | real numbers only (the statement `domain(real)` does the same) |
| `-q`, `--quaternions` | quaternions: `j` and `k` are units next to `i`, the unknown of an equation is a quaternion (`domain(quaternion)`) |
| `-n`, `--numeric` | print numeric values instead of exact forms |
| `-e`, `--exact` | print exact forms only, without the numeric line |
| `--functions` | list the functions with all their names |
| `--constants` | list the constants with all their names |
| `--units` | list the units with all their names and their sizes |
| `-h`, `--help`, `-V`, `--version` | help and version |

What a statement does depends on its form:

- an expression is simplified;
- an equation is solved for its unknown (`x`, `y`, `z`, `t`, `u`, `v`, `w` are preferred, then
  the other names in alphabetical order; `solve(equation, a)` names it);
- equations separated by commas are solved as a system;
- an inequality in one variable is solved over the real numbers;
- `name := value` and `f(x) := expression` define a variable and a function.

Results go to the standard output; a line that starts with `  ~` is the numeric value of the
line above, a line that starts with `note:` is a remark. Errors go to the standard error and
make the exit status 1.

Where the program runs for strangers, as behind a web page, the environment variable
`SYMBOLIC_MATH_SANDBOX=N` keeps the files that it reads and writes under plain names in the
current directory, 64 files and `N` megabytes at most.

`symbolic-math --help` prints, after the options, an example of every kind of operation: every
function and every constant of the program is in at least one of its lines, each line can be
pasted after `symbolic-math`, and the tests run them all.

```
-d 50 'pi'                                  pi
                                              ~ 3.1415926535897932384626433832795028841971693993751
-n 'sqrt(2)+pi'                             4.5558062159628882872643321074892
-e 'x^2=2'                                  x = -sqrt(2)
                                            x = sqrt(2)
'a := 5' 'f(x) := x^2 + a' 'f(3)' "f'(x)"   5
                                            f(x) := x^2 + a
                                            14
                                            2x
'1 + 1; ans * 3'                            2
                                            6
```

## What it does

### Simplifying

```
'10*x-(2+3)*x'        5x
'i^2'                 -1
'(x^2-1)/(x-1)'       x + 1
'sin(x)^2+cos(x)^2'   1
'sqrt(12)+sqrt(27)'   5sqrt(3)
                        ~ 8.6602540378443864676372317075294
'1/(sqrt(2)+1)'       sqrt(2) - 1
                        ~ 0.4142135623730950488016887242097
'(1+i)^10'            32i
'e^(i*pi)'            -1
'cos(pi/5)'           1/4 + sqrt(5)/4
                        ~ 0.80901699437494742410229341718282
'ln(8)/ln(2)'         3
'phi^2-phi-1'         0
```

Commands choose a form: `expand`, `factor`, `together`, `cancel`, `apart`, `collect`,
`trigexpand`, `trigreduce`, `rectform`, `polarform`, `rewrite_exp`, `subst`, and `coeff`,
`degree`, `quo`, `rem`, `gcd`, `lcm`, `resultant`, `discriminant` for polynomials.

```
'expand((a+b)^3)'                     a^3 + 3a^2*b + 3a*b^2 + b^3
'factor(x^4-1)'                       (x + 1)*(x - 1)*(x^2 + 1)
'factor(x^3*y - x*y^3 + x^2 - y^2)'   (x + y)*(x - y)*(x*y + 1)
'factor(2^64+1)'                      274177*67280421310721
'together(1/x+1/y)'                   (x + y)/(x*y)
'apart(1/(x^2-1))'                    1/(2(x - 1)) - 1/(2(x + 1))
'collect(a*x + b*x + c, x)'           c + x*(a + b)
'trigexpand(sin(2x))'                 2cos(x)*sin(x)
'trigreduce(cos(x)^2)'                cos(2x)/2 + 1/2
'rectform(e^(i*pi/4))'                sqrt(2)/2 + i*sqrt(2)/2
'polarform(1+i)'                      sqrt(2)*e^(i*pi/4)
'subst(x^2+y, x, 3)'                  y + 9
'gcd(x^2-1, x^2-2x+1)'                x - 1
'discriminant(a*x^2 + b*x + c, x)'    b^2 - 4a*c
```

### Numbers and precision

Whole numbers, fractions and decimals are exact; `i` (or `j`) is the imaginary unit. A number
written with a decimal point or an exponent makes the answer a decimal one. `N(expr, digits)`
gives a numeric value at any time.

```
'2^100'                                       1267650600228229401496703205376
'1/3+1/6'                                     1/2
                                                ~ 0.5
'0.1+0.2'                                     0.3
'1/3.0'                                       0.33333333333333333333333333333333
'1.234567890123456789012345678901234567891*2'
                                              2.469135780246913578024691357802469135782
'N(pi, 60)'                                   3.14159265358979323846264338327950288419716939937510582097494
'N(zeta(1/2+14i))'                            0.022241142609993589246213199203969 - 0.10325812326645005790236309555257i
'2^(10^9)'                                    2^1000000000
                                                ~ 4.6129760011690693931161192210373e301029995
```

### Equations

Polynomials are solved exactly where radicals or trigonometric forms exist, numerically
otherwise (`x ~ value`). Equations with periodic solutions give every solution, with whole
parameters. Each root is checked numerically before it is printed.

A solution must give the equation a value: `x*ln(x) = 0` has the one solution 1, and a family
leaves out the whole numbers for which the equation has none (`sin(x)/x = 0`), one by one or
as whole classes (`sin(2x)/sin(x) = 0`). Common polynomial factors cancel first, so
`(x^2 - 1)/(x - 1) = 2` has the solution 1. What was assumed about the unknown (`assume(x,
positive)`, `real`, `integer`, ...) leaves only the solutions of that kind. `ans` and `s :=
solve(...)` hold the solutions as a list.

```
'x^2-5x+6=0'                              x = 2
                                          x = 3
'x^3=8'                                   x = 2
                                          x = -1 - i*sqrt(3)
                                            ~ -1 - 1.7320508075688772935274463415059i
                                          x = -1 + i*sqrt(3)
                                            ~ -1 + 1.7320508075688772935274463415059i
'x^3-3x+1=0'                              x = -2cos(pi/9)
                                            ~ -1.8793852415718167681082185546495
                                          x = 2cos(4pi/9)
                                            ~ 0.34729635533386069770343325353863
                                          x = 2cos(2pi/9)
                                            ~ 1.5320888862379560704047853011108
'(x-1)^2*(x+2)=0'                         x = -2
                                          x = 1 (multiplicity 2)
'a*x^2+b*x+c=0'                           x = (-b - sqrt(b^2 - 4a*c))/(2a)
                                          x = (sqrt(b^2 - 4a*c) - b)/(2a)
'x^5-x-1=0'                               x ~ 1.1673039782614186842560458998548
                                          x ~ -0.76488443360058472602982318770854 - 0.35247154603172624931794709140258i
                                          x ~ -0.76488443360058472602982318770854 + 0.35247154603172624931794709140258i
                                          x ~ 0.18123244446987538390180023778112 - 1.0839541013177106684303444929808i
                                          x ~ 0.18123244446987538390180023778112 + 1.0839541013177106684303444929808i
'x^5+x=i'                                 x = -sqrt(3)/2 + i/2
                                            ~ -0.86602540378443864676372317075294 + 0.5i
                                          x = -sqrt(-1/3 + (11/54 + sqrt(69)/18)^(1/3)/2 - (sqrt(69)/18 - 11/54)^(1/3)/2 + i*sqrt(3)*((11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))/2)
                                            ~ -0.74486176661974423659317042860439 - 0.87743883312334638002475444817926i
                                          x = sqrt(-1/3 - (11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))
                                            ~ 0.75487766624669276004950889635853i
                                          x = sqrt(-1/3 + (11/54 + sqrt(69)/18)^(1/3)/2 - (sqrt(69)/18 - 11/54)^(1/3)/2 - i*sqrt(3)*((11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))/2)
                                            ~ 0.74486176661974423659317042860439 - 0.87743883312334638002475444817926i
                                          x = sqrt(3)/2 + i/2
                                            ~ 0.86602540378443864676372317075294 + 0.5i
'cos(x)=1/2'                              x = -pi/3 + 2n*pi, n is an integer
                                          x = pi/3 + 2k*pi, k is an integer
'sin(x)+cos(x)=1'                         x = 2k*pi, k is an integer
                                          x = pi/2 + 2n*pi, n is an integer
'2^x=8'                                   x = 3 + 2i*k*pi/ln(2), k is an integer
'x*e^x=1'                                 x = lambertw(1)
                                            ~ 0.56714329040978387299996866221036
                                          note: Lambert W: the real branches are given; lambertw(k, z) with other integers k gives complex solutions
'sqrt(x)+sqrt(x+5)=5'                     x = 4
'solve(x^2+y^2=1, y)'                     y = -sqrt(1 - x^2)
                                          y = sqrt(1 - x^2)
'x^2 = 2 ± 1'                             x = -sqrt(3)
                                            ~ -1.7320508075688772935274463415059
                                          x = sqrt(3)
                                            ~ 1.7320508075688772935274463415059
                                          x = -1
                                          x = 1
'nsolve(cos(x) = x, x)'                   0.73908513321516064165531208767387
'sin(x)/x = 0'                            x = k*pi, k is an integer other than 0
'cot(x) = 0'                              x = pi/2 + n*pi, n is an integer
'sin(2x)/sin(x) = 0'                      x = pi/2 + k*pi, k is an integer
'x*sin(x) = sin(x)'                       x = 1
                                          x = k*pi, k is an integer
'(x - pi/2)*tan(x) = 0'                   x = k*pi, k is an integer
'assume(x, positive); x^2 = 4'            true
                                          x = 2
'assume(n, integer); 6n^2 - 5n + 1 = 0'   true
                                          no integer solution
'x^2 = 2; N(ans)'                         x = -sqrt(2)
                                            ~ -1.4142135623730950488016887242097
                                          x = sqrt(2)
                                            ~ 1.4142135623730950488016887242097
                                          [x = -1.4142135623730950488016887242097, x = 1.4142135623730950488016887242097]
's := solve(x^3 = 1, x); s'               x = 1
                                          x = -1/2 - i*sqrt(3)/2
                                            ~ -0.5 - 0.86602540378443864676372317075294i
                                          x = -1/2 + i*sqrt(3)/2
                                            ~ -0.5 + 0.86602540378443864676372317075294i
                                          [x = 1, x = -1/2 - i*sqrt(3)/2, x = -1/2 + i*sqrt(3)/2]
```

### Systems

```
'x+y=3, x-y=1'                           x = 2, y = 1
'x^2+y^2=25, x+y=7'                      x = 4, y = 3
                                         x = 3, y = 4
'a*x+b*y=1, c*x+d*y=0'                   x = d/(a*d - b*c), y = -c/(a*d - b*c)
'x+y+z=6, x-y+z=2, x+y-z=0'              x = 1, y = 2, z = 3
'x+y=2, 2x+2y=4'                         x = 2 - y
                                         note: free: y (any value)
'solve([x+y=3, x*y=2], [x, y])'          x = 2, y = 1
                                         x = 1, y = 2
'x*y=0, x+y=1'                           x = 0, y = 1
                                         x = 1, y = 0
'solve([x*y=0, x*z=0], [x, y, z])'       x = 0
                                         y = 0, z = 0
                                         note: free: y, z (any value)
                                         note: free: x (any value)
'sin(x)+y=1, x-y=1'                      x = 1.1060601577062719106167372970301, y = 0.10606015770627191061673729703007
                                         x = -29.71469768582627937478337646524 + 4.1582135162052696736721622812845i, y = -30.71469768582627937478337646524 + 4.1582135162052696736721622812845i
                                         x = -4.3165396770471007402706630535392 + 2.6114255442395965953681945547882i, y = -5.3165396770471007402706630535392 + 2.6114255442395965953681945547882i
                                         x = 3.9868981342821752127605526449077 + 1.6323448990594223732641651728579i, y = 2.9868981342821752127605526449077 + 1.6323448990594223732641651728579i
                                         x = 10.670593855146943920308018388534 + 2.9038473037758443345039936265515i, y = 9.6705938551469439203080183885341 + 2.9038473037758443345039936265515i
                                         note: numeric search: the roots found are listed, there may be others
'exp(x)*sin(y) = 1, exp(x)*cos(y) = 1'   x = 0.34657359027997265470861606072909, y = 0.78539816339744830961566084581988
                                         x = 0.34657359027997265470861606072909 - 3.1415926535897932384626433832795i, y = -2.3561944901923449288469825374596
                                         x = 0.34657359027997265470861606072909 + 3.1415926535897932384626433832795i, y = -2.3561944901923449288469825374596
                                         x = 0.34657359027997265470861606072909 - 3.1415926535897932384626433832795i, y = 3.9269908169872415480783042290994
                                         x = 0.34657359027997265470861606072909 + 3.1415926535897932384626433832795i, y = 3.9269908169872415480783042290994
                                         x = 0.34657359027997265470861606072909, y = 7.0685834705770347865409476123789
                                         note: found by a numeric search from many starting points: there may be other solutions
'sin(x) = 0, cos(x) = 1'                 x = 2k*pi, k is an integer
'sin(x) = 0, x^2 = pi^2'                 x = -pi
                                           ~ -3.1415926535897932384626433832795
                                         x = pi
                                           ~ 3.1415926535897932384626433832795
```

A product that must vanish gives a case for each factor. A system for which no exact method
finds anything is searched numerically from many starting points; a note tells that. Several
equations in one unknown have the roots of one of them that solve the others too.

### Inequalities

An inequality in one variable, or several joined by commas or chained, is solved over the real
numbers.

```
'x^2-4>0'          x < -2 or x > 2
'(x-1)/(x+2)<=0'   -2 < x <= 1
'x^2>4, x<5'       x < -2 or 2 < x < 5
'abs(x)<3'         -3 < x < 3
'sqrt(x)<2'        0 <= x < 4
'sin(x)>=1/2'      pi/6 + 2k*pi <= x <= 5pi/6 + 2k*pi, k is an integer
'tan(x)>1'         pi/4 + k*pi < x < pi/2 + k*pi, k is an integer
'pi>=3'            true
```

`minimize` and `maximize` give the extreme value of a function of one real variable and the
points where it is taken, on the whole line or between two bounds.

```
'minimize(x^2-2x+3)'           [2, x = 1]
'minimize(x^4-2x^2)'           [-1, x = -1, x = 1]
'maximize(x*e^(-x))'           [1/e, x = 1]
'minimize(x+1/x, x, 0, inf)'   [2, x = 1]
'minimize(x^3)'                -inf
                               note: x^3 is unbounded below (x -> -inf)
```

### Derivatives, integrals, limits, series, sums

```
'diff(sin(x)^2, x)'                           2cos(x)*sin(x)
'diff(x^3*y^2, x, y)'                         6x^2*y
'diff(x^5, x, 3)'                             60x^2
'd/dx x^x'                                    x^x*(ln(x) + 1)
'grad(x^2*y+z)'                               [2x*y, x^2, 1]
'integrate(x*e^x, x)'                         e^x*(x - 1)
'integrate(1/(x^3+1), x)'                     ln(x + 1)/3 - ln(x^2 - x + 1)/6 + sqrt(3)*atan(sqrt(3)*(2x - 1)/3)/3
'integrate(e^(-x^2), x)'                      sqrt(pi)*erf(x)/2
'integrate(e^(-x^2), x, -inf, inf)'           sqrt(pi)
                                                ~ 1.7724538509055160272981674833411
'integrate(sin(x)/x, x, 0, inf)'              pi/2
                                                ~ 1.5707963267948966192313216916398
'integrate(abs(x^2-1), x, -2, 2)'             4
'integrate(e^(sin(x)), x, 0, 1)'              1.6318696084180513481371617237447
'limit(sin(x)/x, x, 0)'                       1
'limit((1+a/x)^x, x, inf)'                    e^a
'limit(1/x, x, 0, +)'                         inf
'series(sin(x), x, 0, 7)'                     x - x^3/6 + x^5/120 - x^7/5040 + O(x^8)
'sum(k^2, k, 1, n)'                           n^3/3 + n^2/2 + n/6
'sum(1/k^2, k, 1, inf)'                       pi^2/6
                                                ~ 1.644934066848226436472415166646
'sum(1/k!, k, 0, inf)'                        e
                                                ~ 2.7182818284590452353602874713527
'sum((-1)^k/(2k+1), k, 0, inf)'               pi/4
                                                ~ 0.78539816339744830961566084581988
'sum(1/(k^2+k), k, 1, n)'                     n/(n + 1)
'sum(binomial(n,k)*x^k, k, 0, n)'             (x + 1)^n
'product(k, k, 1, n)'                         n!
'product((k+1)/k, k, 1, n)'                   n + 1
'product(1-1/(4k^2), k, 1, inf)'              2/pi
                                                ~ 0.63661977236758134307553505349006
'product(1+1/k^2, k, 1, inf)'                 sinh(pi)/pi
                                                ~ 3.6760779103749777206956974920283
'gamma(n+3)/gamma(n)' 'gamma(x)*gamma(1-x)'   n^3 + 3n^2 + 2n
                                              pi/sin(pi*x)
'integrate(ln(x)/(1-x), x)'                   li2(1 - x)
'residue(e^z/z^3, z, 0)' 'seriescoeff(tan(x), x, 7)'
                                              1/2
                                                ~ 0.5
                                              17/315
                                                ~ 0.053968253968253968253968253968254
'pade(e^x, x, 2, 2)'                          (x^2 + 6x + 12)/(x^2 - 6x + 12)
'criticalpoints(x^3 - 3x*y + y^3, [x, y])'    [[[0, 0], 0, saddle], [[1, 1], -1, minimum]]
```

An antiderivative is checked by differentiation before it is printed; an integral with no
closed form found is given back as it is, a definite one is then calculated numerically.
A product of a quotient of polynomials (and of powers) has a closed form in gamma functions,
also without an upper end; quotients of gamma functions and factorials whose arguments differ
by a whole number, and `gamma(u)*gamma(1 - u)`, are reduced.
Also: `hessian`, `laplacian`, `jacobian`, `divergence`, `curl`, `nintegrate`.

### Differential equations and recurrences

`dsolve(equation, y, x)` solves linear equations with constant coefficients of any order,
first-order linear, separable, Bernoulli, homogeneous and exact equations and Cauchy-Euler
equations; `y'`, `y''`, `diff(y, x)` and `y'(x)` are all accepted. Conditions `y(a) = b`,
`y'(a) = b` in a list fix the constants. `rsolve` solves linear recurrences with constant
coefficients. `dsolve([equations], [x, y], t)` solves a linear system with constant
coefficients, also with a right side and with start values. Equations of the kinds of
Bessel's and Airy's are solved with those functions. `ndsolve([equations, start values], y,
[x, a, b])` follows any system by numbers (an extrapolation method) and gives the value at `b`
with all the digits asked for; `[x, a, b, n]` gives the values at `n` steps.

```
"dsolve(y' = y, y, x)"                        y = C1*e^x
"dsolve(y'' + y = sin(x))"                    y = C1*cos(x) + C2*sin(x) - x*cos(x)/2
"dsolve([y'' + y = 0, y(0) = 1, y'(0) = 2], y, x)"
                                              y = cos(x) + 2sin(x)
"dsolve(y' + y/x = x, y, x)"                  y = x^2/3 + C1/x
"dsolve(y' = x/y)"                            y = -sqrt(x^2 + C1)
                                              y = sqrt(x^2 + C1)
"dsolve(x^2*y'' + x*y' - y = 0)"              y = C2*x + C1/x
"dsolve(y'' + w^2*y = 0, y, x)"               y = C1*cos(w*x) + C2*sin(w*x)
'rsolve([a(n+2) = a(n+1) + a(n), a(0) = 0, a(1) = 1], a)'
                                              a(n) = -sqrt(5)*((1/2 - sqrt(5)/2)^n - (1/2 + sqrt(5)/2)^n)/5
'rsolve([a(n) = 3a(n-1) + 2^n, a(0) = 1], a)'
                                              a(n) = 3*3^n - 2*2^n
"dsolve([x'(t) = y(t), y'(t) = -x(t), x(0) = 1, y(0) = 0], [x, y], t)"
                                              x = cos(t)
                                              y = -sin(t)
"dsolve([x'(t) = 3x(t) - y(t) + e^t, y'(t) = x(t) + y(t)], [x, y], t)"
                                              x = e^(2t)*(C1*t - C2*t + C1)
                                              y = e^t*(C1*t*e^t - C2*t*e^t + C2*e^t + 1)
"dsolve(x^2*y'' + x*y' + (x^2 - 4)*y = 0, y, x)"
                                              y = C1*besselj(2, x) + C2*bessely(2, x)
"dsolve(y'' - x*y = 0, y, x)"                 y = C1*airyai(x) + C2*airybi(x)
"ndsolve([y' = x^2 + y^2, y(0) = 0], y, [x, 0, 1])"
                                              y(1) = 0.35023184431675577784938227410474
"ndsolve([u' = v, v' = -u, u(0) = 1, v(0) = 0], [u, v], [t, 0, 1])"
                                              u(1) = 0.54030230586813971740093660744298
                                              v(1) = -0.8414709848078965066525023216303
-d 50 "ndsolve([y' = y, y(0) = 1], y, [x, 0, 1])"
                                              y(1) = 2.7182818284590452353602874713526624977572470937
```

### Matrices and vectors

A list `[a, b, c]` is a vector; `[a, b; c, d]` or `[[a, b], [c, d]]` is a matrix. Sums and
products follow the rules of linear algebra; a function of one argument applies to each entry.

```
'[1,2;3,4]*[5,6;7,8]'         [[19, 22], [43, 50]]
'[1,2;3,4]^-1'                [[-2, 1], [3/2, -1/2]]
'det([a,b;c,d])'              a*d - b*c
'eigenvalues([1,2;3,4])'      [5/2 - sqrt(33)/2, 5/2 + sqrt(33)/2]
'eigenvectors([2,1;1,2])'     [[1, [[-1, 1]]], [3, [[1, 1]]]]
'charpoly([1,2;3,4])'         lambda^2 - 5lambda - 2
'rref([1,2,3;4,5,6])'         [[1, 0, -1], [0, 1, 2]]
'nullspace([1,2;2,4])'        [[-2, 1]]
'linsolve([1,2;3,4],[5,6])'   [-4, 9/2]
'cross([1,0,0],[0,1,0])'      [0, 0, 1]
'sqrt([1, 4, 9])'             [1, 2, 3]
```

### Whole numbers

```
'factor(10^30+1)'                             61*101*3541*9901*27961*4188901*39526741
'isprime(2^127-1)'                            true
'prime(100)'                                  541
'primepi(1000)'                               168
'divisors(28)'                                [1, 2, 4, 7, 14, 28]
'totient(100)'                                40
'fibonacci(50)'                               12586269025
'binomial(10, 3)'                             120
'bernoulli(10)'                               5/66
                                                ~ 0.075757575757575757575757575757576
'partitions(100)'                             190569292
'powmod(2, 100, 97)'                          16
'crt([2,3,2],[3,5,7])'                        23
'cfrac(pi, 6)'                                [3, 7, 15, 1, 292, 1]
'convergents(pi, 4)'                          [3, 22/7, 333/106, 355/113]
'17 mod 5'                                    2
'sqrtmod(4, 15)' 'sqrtmodall(1, 24)'          2
                                              [1, 5, 7, 11, 13, 17, 19, 23]
'discretelog(3, 3^(2^60) mod (2^127-1), 2^127-1)'
                                              1152921504606846976
'carmichael(561)' 'primes(100, 130)'          80
                                              [101, 103, 107, 109, 113, 127]
'pell(61)'                                    [1766319049, 226153980]
'squaresr(4, 10^20)'                          2861022949218744
```

`modsolve` solves a polynomial equation modulo a number (prime powers by lifting, a large
prime by the common factor with `x^p - x`), and `diophantine` solves in whole numbers: linear
equations in any number of unknowns, `x y` forms, sums and differences of two squares, Pell's
equation, and a search in a box otherwise.

```
'modsolve(x^2 = 2, x, 10^9+7)'               [59713600, 940286407]
'modsolve(x^3 = 1, x, 1000003)'              [1, 499501, 500501]
'modsolve(x^2 = 1, x, 2^4*3^2)'              [1, 17, 55, 71, 73, 89, 127, 143]
'diophantine(3x + 5y = 1, [x, y])'           [x -> 5t + 2, y -> -3t - 1]
                                             note: t is any whole number
'diophantine(2x + 3y + 5z = 7, [x, y, z])'   [x -> t1, y -> -4t1 + 5t2 + 14, z -> 2t1 - 3t2 - 7]
                                             note: t1, t2 are any whole numbers
'diophantine(x^2 - 61y^2 = 1, [x, y])'       [x -> (1766319049 + 226153980sqrt(61))^n/2 + (1766319049 - 226153980sqrt(61))^n/2, y -> sqrt(61)*((1766319049 + 226153980sqrt(61))^n - (1766319049 - 226153980sqrt(61))^n)/122]
                                             note: n is any whole number; with every solution both signs changed is one too; the least solution of x^2 - 61 y^2 = 1 is (1766319049, 226153980)
'diophantine(x^2 + y^2 = 25, [x, y])'        [[-5, 0], [-4, -3], [-4, 3], [-3, -4], [-3, 4], [0, -5], [0, 5], [3, -4], [3, 4], [4, -3], [4, 3], [5, 0]]
```

### Special functions

Gamma, zeta and error functions, exponential and trigonometric integrals, polylogarithms,
Lambert W, the incomplete gamma and beta functions, Bessel functions (`besselj`, `bessely`,
`besseli`, `besselk`, Hankel and spherical ones), Airy and Struve functions, hypergeometric
functions (`hyp0f1`, `hyp1f1`, `hyperu`, `hyp2f1` and the general `hypergeometric`), elliptic
integrals, Jacobi's elliptic and theta functions, the orthogonal polynomials, spherical
harmonics, the step and delta functions: they have exact values and closed forms where those
are known, derivatives, and numeric values for real and complex arguments with any number of
digits. The numeric values were compared with mpmath in all four quadrants.

```
'gamma(1/2)' 'zeta(4)' 'li2(1/2)'             sqrt(pi)
                                                ~ 1.7724538509055160272981674833411
                                              pi^4/90
                                                ~ 1.0823232337111381915160036965412
                                              pi^2/12 - ln(2)^2/2
                                                ~ 0.58224052646501250590265632015968
'N(gamma(1/3+2i))' 'N(erf(1+i))'              0.062959426851031394514118228764171 - 0.073342946031507627850028125931902i
                                              1.3161512816979476448802710802437 + 0.19045346923783468628410886196916i
'lambertw(1)'                                 lambertw(1)
                                                ~ 0.56714329040978387299996866221036
'erfc(100)'                                   erfc(100)
                                                ~ 6.4059614249217320390213391485864e-4346
'besselj(1/2, x)' 'diff(besselj(0, x), x)'    sqrt(2)*sin(x)/(sqrt(pi)*sqrt(x))
                                              -besselj(1, x)
'N(besselj(0, 1+i))'                          0.93760847680602927659973819742582 - 0.49652994760912213216645972122524i
-d 50 'besselk(1/3, 2)'                       besselk(1/3, 2)
                                                ~ 0.11654496129616524875894262891463289675317417894374
'airyai(0)'                                   3^(1/3)/(3gamma(2/3))
                                                ~ 0.35502805388781723926006318600418
'hyp2f1(1, 1, 2, x)' 'hypergeometric([1], [], x)'
                                              -ln(1 - x)/x
                                              1/(1 - x)
'elliptick(1/2)' 'jacobisn(u, 1)'             gamma(1/4)^2/(4sqrt(pi))
                                                ~ 1.8540746773013719184338503471953
                                              tanh(u)
'legendrep(3, x)' 'sphericalharmonicy(1, 1, t, p)'
                                              5x^3/2 - 3x/2
                                              -sqrt(6)*e^(i*p)*sin(t)/(4sqrt(pi))
'integrate(legendrep(2, x)^2, x, -1, 1)'      2/5
                                                ~ 0.4
'uppergamma(2, x)' 'erf(erfinv(x))'           e^(-x)*(x + 1)
                                              x
'integrate(diracdelta(x-1)*x^2, x, -inf, inf)'
                                              1
```

### Closed forms of numbers

```
'identify(1.6449340668482264364724151666460)'
                                              pi^2/6
                                                ~ 1.644934066848226436472415166646
'identify(0.3678794411714423215955237701615)'
                                              1/e
                                                ~ 0.36787944117144232159552377016146
'identify(4.1415926535897932384626433832795)'
                                              1 + pi
                                                ~ 4.1415926535897932384626433832795
'rationalize(pi, 0.001)'                      333/106
                                                ~ 3.1415094339622641509433962264151
'minpoly(sqrt(2)+sqrt(3), x)'                 x^4 - 10x^2 + 1
                                              note: the polynomial was found with numbers and confirmed with twice the digits
'findrelation([ln(2), ln(3), ln(6)])'         [1, 1, -1]
'latticereduce([[1,1,1],[-1,0,2],[3,5,6]])'   [[0, 1, 0], [1, 0, 1], [-1, 0, 2]]
```

`minpoly` and `findrelation` look for whole numbers by lattice reduction (`latticereduce`, the
LLL algorithm) with many digits, and confirm what they find with twice as many.

### Integral transforms

`laplace`, `invlaplace`, `fourier` (the integral of `f(x) e^(-i k x)`), `invfourier`,
`fourierseries`, `ztransform` and `invztransform`. A parameter whose sign matters needs an
assumption, as in `assume(a > 0)`.

```
'laplace(t*sin(t), t, s)'              2s/(s^2 + 1)^2
'laplace(besselj(0, t), t, s)'         1/sqrt(s^2 + 1)
'invlaplace((s+3)/(s^2+2s+5), s, t)'   e^(-t)*(cos(2t) + sin(2t))
'invlaplace(e^(-2s)/s, s, t)'          heaviside(t - 2)
'fourier(e^(-x^2), x, w)'              sqrt(pi)*e^(-w^2/4)
'fourier(1/(1+x^2), x, w)'             pi*e^(-abs(w))
'invfourier(e^(-w^2/4), w, x)'         e^(-x^2)/sqrt(pi)
'fourierseries(abs(x), x, 3)'          pi/2 - 4cos(x)/pi - 4cos(3x)/(9pi)
'ztransform(n^2, n, z)'                z*(z + 1)/(z - 1)^3
'invztransform(z/(z-2), z, n)'         2^n
```

### Lists, programs and logic

A list holds any values. There are commands that build lists (`range`, `table`, `array`), pick
from them (`part` or `list[[i]]`, `take`, `drop`, `first`, `last`), order and combine them
(`sort`, `reverse`, `join`, `union`, `zip`, `permutations`, `subsets`), and commands that take
a function, written `x -> expression` or as the name of a function (`map`, `select`, `fold`,
`nest`, `nestlist`, `fixedpoint`). `if`, `piecewise`, `while`, `do`, `for` and `block` make
small programs; `and`, `or`, `not`, `xor`, `implies` and the comparisons give `true` or
`false` for exact numbers, with all the digits that the decision needs.

```
'table(k^2, k, 1, 5)'                         [1, 4, 9, 16, 25]
'map(x -> x^2, [1, 2, 3])' 'select(range(20), isprime)'
                                              [1, 4, 9]
                                              [2, 3, 5, 7, 11, 13, 17, 19]
'fold((a, b) -> a + b, 0, [1, 2, 3])' 'nestlist(x -> 2x, 1, 5)'
                                              6
                                              [1, 2, 4, 8, 16, 32]
'sort([3, pi, e, sqrt(2)])' '[a, b, c][[2]]'
                                              [sqrt(2), e, 3, pi]
                                              b
'subsets([1, 2, 3], 2)'                       [[1, 2], [1, 3], [2, 3]]
'f(x) := piecewise(x < 0, -x, x^2); f(-2); f(3); diff(f(x), x)'
                                              f(x) := piecewise(x < 0, -x, x^2)
                                              2
                                              9
                                              piecewise(x < 0, -1, 2x)
'k := 1; while(k < 100, k := 2k); k'          1
                                              128
                                              128
's := 0; do(s := s + k^2, [k, 1, 10]); s'     0
                                              385
                                              385
'if(isprime(2^61-1), yes, no)'                yes
'[x + y, x*y] = [3, 2]'                       x = 2, y = 1
                                              x = 1, y = 2
'isrational(sqrt(2))' 'e^pi > pi^e'           false
                                              true
```

### Statistics and probability

Data in lists: `mean`, `median`, `mode`, `variance`, `stddev` (and `pvariance`, `pstddev` over
n), `covariance`, `correlation`, `quantile`, `quartiles`, `iqr`, `moment`, `centralmoment`,
`skewness`, `kurtosis`, `geometricmean`, `harmonicmean`, `rms`, `meandeviation`.
Distributions: `normaldist`, `uniformdist`, `exponentialdist`, `gammadist`, `chisquaredist`,
`studenttdist`, `betadist`, `cauchydist`, `lognormaldist`, `weibulldist`, `laplacedist`,
`rayleighdist`, `logisticdist`, `paretodist`, `fdist`, and the discrete `binomialdist`,
`poissondist`, `geometricdist`, `bernoullidist`, `discreteuniformdist`,
`negativebinomialdist`, `hypergeometricdist`, with `pdf`, `cdf`, `quantile`, `mean`,
`variance`, `stddev`, `median`, `mode`, `skewness`, `kurtosis`, `mgf`, `expectation` and
`probability`; their parameters may be symbols.

```
'mean([1,2,3,4])' 'variance([1,2,3,4])' 'median([3,1,2])'
                                              5/2
                                                ~ 2.5
                                              5/3
                                                ~ 1.6666666666666666666666666666667
                                              2
'correlation([1,2,3],[2,4,7])'                5sqrt(57)/38
                                                ~ 0.99339926779878285489956379038765
'quartiles([1,2,3,4,5,6,7,8])'                [11/4, 9/2, 25/4]
'pdf(normaldist(m, s), x)'                    sqrt(2)*e^(-(x - m)^2/(2s^2))/(2sqrt(pi)*s)
'cdf(normaldist(0, 1), 1)'                    1/2 + erf(sqrt(2)/2)/2
                                                ~ 0.84134474606854294858523254563204
'quantile(studenttdist(5), 0.975)'            2.5705818356363155146962462174396
'probability(x > 1, x, exponentialdist(1))'   1/e
                                                ~ 0.36787944117144232159552377016146
'expectation(x^2, x, normaldist(0, 1))'       1
'pdf(binomialdist(10, 1/3), 4)'               4480/19683
                                                ~ 0.22760758014530305339633185998069
'variance(poissondist(l))' 'mean(geometricdist(p))'
                                              l
                                              1/p - 1
```

### Interpolation, fitting and numeric roots

```
'interpolate([[0,1],[1,3],[2,7]], x)'         x^2 + x + 1
'fit([[1,2],[2,4.1],[3,5.9]], [1, x], x)'     1.95x + 0.1
'leastsquares([[1,1],[1,2],[1,3]], [1,2,2])'
                                              [2/3, 1/2]
'nsolve([x^2 + y^2 = 4, x*y = 1], [x, y], [2, 0.5])'
                                              [x -> 1.9318516525781365734994863994578, y -> 0.5176380902050415246977976752481]
'criticalpoints(x^2 - y^2, [x, y])'           [[[0, 0], 0, saddle]]
```

### Assumptions

Symbols are complex unless something else is assumed.

```
'sqrt(x^2)'                                   sqrt(x^2)
'assume(x, real)' 'sqrt(x^2)'                 true
                                              abs(x)
'assume(x > 0)' 'sqrt(x^2)'                   true
                                              x
'assume(n, integer)' 'sin(n*pi)' 'cos(n*pi)'
                                              true
                                              0
                                              (-1)^n
```

### Real numbers only

With `-r` (or after the statement `domain(real)`) every symbol is a real number, only real
solutions are given, and odd roots of negative numbers are real. A result that is not a real
number is `undefined`, with a note that names its complex value. `domain(complex)` goes back.

```
-r 'x^2+1=0'                                  no real solution
-r 'x^3=-8'                                   x = -2
-r '(-8)^(1/3)'                               -2
-r 'e^x=1'                                    x = 0
-r 'sqrt(x^2)'                                abs(x)
-r 'integrate(cbrt(x), x, -8, 1)'             -45/4
                                                ~ -11.25
-r 'sqrt(-4)'                                 undefined
                                              note: the value 2i is not a real number
'domain(real); x^4=16; domain(complex); x^4=16'
                                              real
                                              x = -2
                                              x = 2
                                              complex
                                              x = -2
                                              x = 2
                                              x = -2i
                                              x = 2i
```

### Quaternions

With `-q` (or `domain(quaternion)`) `j` and `k` are quaternion units next to `i`: `i*j = k`,
`j*i = -k`. Products keep the order they are written in; symbols stand for real numbers.
`quat(a, b, c, d)` is `a + b i + c j + d k` in every domain. Powers, `conj`, `abs`, `inv`,
`exp`, `ln`, `sqrt` and the trigonometric functions take quaternions.

```
-q 'i*j' 'j*i' 'i*j*k'                        k
                                              -k
                                              -1
-q '(1+2i+3j+4k)*(5+6i+7j+8k)'                -60 + 12i + 30j + 24k
-q '1/(1+2i+3j+4k)'                           1/30 - i/15 - j/10 - 2k/15
-q 'abs(1+2i+3j+4k)'                          sqrt(30)
                                                ~ 5.477225575051661134569697828008
-q 'exp(j*pi)' 'ln(j)' 'sqrt(j)'              -1
                                              pi*j/2
                                              sqrt(2)/2 + sqrt(2)*j/2
-q '(a+b*i+c*j+d*k)*(a-b*i-c*j-d*k)'          a^2 + b^2 + c^2 + d^2
-q 'rotate([1,0,0], rotationquat(pi/2, [0,0,1]))'
                                              [0, 1, 0]
'quat(1,2,3,4)^2'                             quat(-28, 4, 6, 8)
```

#### Equations in quaternions

In the quaternion domain the unknown of an equation is a quaternion `x0 + x1 i + x2 j + x3 k`
and does not commute with the other quaternions of the equation (the other symbols stay real
numbers). An equation with real coefficients has whole spheres of solutions: `u` stands for
any unit pure quaternion.

```
-q 'x^2 = -1'                       x = u, u is any unit pure quaternion
                                    note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1
-q 'x^2 + x + 1 = 0'                x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion
                                      ~ 0.86602540378443864676372317075294u - 0.5
                                    note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1
-q 'i*x = j' 'x*i = j'              x = -k
                                    x = k
-q 'x*i - i*x = j'                  x = x0 + x1*i + k/2
                                    x0 and x1 are any real numbers
-q '2x + i*x*j = 1 + j'             x = 2/3 + i/3 + 2j/3 - k/3
                                      ~ 0.66666666666666666666666666666667 + 0.33333333333333333333333333333333i + 0.66666666666666666666666666666667j - 0.33333333333333333333333333333333k
-q 'x^2 = 1 + j'                    x = -sqrt(1/2 + sqrt(2)/2) - sqrt(sqrt(2)/2 - 1/2)*j
                                      ~ -1.0986841134678099660398011952407 - 0.45508986056222734130435775782247j
                                    x = sqrt(1/2 + sqrt(2)/2) + sqrt(sqrt(2)/2 - 1/2)*j
                                      ~ 1.0986841134678099660398011952407 + 0.45508986056222734130435775782247j
-q 'exp(x) = j'                     x = (pi/2 + 2n*pi)*j, n is an integer
-q 'x^2 + i*x + j = 0'              x = -1/2 - i/2 + j/2 + k/2
                                      ~ -0.5 - 0.5i + 0.5j + 0.5k
                                    x = 1/2 - i/2 - j/2 + k/2
                                      ~ 0.5 - 0.5i - 0.5j + 0.5k
-q 'x^3 + i*x = j'                  x ~ -0.8808438273383357247011884027713 + 0.46533216880704412441983208161383i + 0.41301567882709347798449350099886j - 0.21818791891937072571575101281653k
                                    x ~ -0.46557123187676802665673122521994j - 0.68232780382801932736948373971105k
                                    x ~ 0.8808438273383357247011884027713 - 0.46533216880704412441983208161383i + 0.41301567882709347798449350099886j - 0.21818791891937072571575101281653k
-q 'x*i*x = j'                      x = -sqrt(2)*i/2 + sqrt(2)*j/2
                                      ~ -0.70710678118654752440084436210485i + 0.70710678118654752440084436210485j
                                    x = sqrt(2)*i/2 - sqrt(2)*j/2
                                      ~ 0.70710678118654752440084436210485i - 0.70710678118654752440084436210485j
-q 'x*i*x = i'                      x = x0 + x2*j + x3*k with x0^2 + x2^2 + x3^2 = 1
                                    x0, x2 and x3 are real numbers
-q 'x^2 = conj(x)'                  x = 0
                                    x = 1
                                    x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion
                                      ~ 0.86602540378443864676372317075294u - 0.5
                                    note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1
-q 'abs(x) = 2'                     x = x0 + x1*i + x2*j + x3*k with sqrt(x0^2 + x1^2 + x2^2 + x3^2) = 2
                                    x0, x1, x2 and x3 are real numbers
-q 'x + y = 1 + j, x - y = i + k'   x = 1/2 + i/2 + j/2 + k/2, y = 1/2 - i/2 + j/2 - k/2
                                      ~ x = 0.5 + 0.5i + 0.5j + 0.5k, y = 0.5 - 0.5i + 0.5j - 0.5k
-q '[x, y] = [1 + j, k]'            x = 1 + j, y = k
-q 'x*(x - i)*(x - j) = 0'          x = 0
                                    x = i
                                    x = j
-q 'x^2 = [-1, 4]'                  x = u, u is any unit pure quaternion
                                    note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1
                                    x = -2
                                    x = 2
-q 'e^x = e^j'                      x = (1 + 2n*pi)*j, n is an integer
'qsolve(i*x = j, x)'                x = -k
```

A product that must vanish is solved factor by factor (the quaternions have no zero
divisors), and a list on one side gives an equation for each entry. The methods for one
equation, in this order: the four components as a linear system (free components are
named `x0` to `x3`); an equation `G(x) = c` with real coefficients on the left is solved in
the complex numbers and carried into the plane of `c`; a polynomial with its quaternion
coefficients on one side is solved through its companion polynomial, which gives every zero;
everything else goes through the four real components, and a note tells when that search may
have missed solutions. Every solution is put back into the equation before it is printed.

An unknown that was assumed real, or that stands inside a `quat(a, b, c, d)`, is a real number
and gets only real values:

```
-q 'assume(x, real); assume(y, real); x*j + y*k = 4j + 2k'
                                              true
                                              true
                                              x = 4, y = 2
-q 'quat(a, b, c, d)*i = j + 2k'              a = 0, b = 0, c = -2, d = 1
-q 'quat(a, b, c, d)^2 = -1'                  a = 0, b = -sqrt(-c^2 - d^2 + 1)
                                              a = 0, b = sqrt(-c^2 - d^2 + 1)
                                              note: free: c, d (any value)
-q 'assume(x, real); x^2 = -1'                true
                                              no real solution
```

### Real and imaginary parts

`complexexpand` writes an expression as real part plus `i` times imaginary part, taking every
symbol as real; `rectform` does it for numbers. `re`, `im`, `abs`, `arg` and `conj` work on
symbolic expressions once their symbols are real.

```
'complexexpand((x+i*y)^3)'           x^3 - 3x*y^2 + i*(3x^2*y - y^3)
'complexexpand(exp(x+i*y))'          e^x*cos(y) + i*e^x*sin(y)
'complexexpand(ln(x+i*y))'           ln(x^2 + y^2)/2 + i*atan2(y, x)
'complexexpand(abs(sin(x+i*y))^2)'   cosh(y)^2 - cos(x)^2
'rectform(asin(2))'                  pi/2 - i*ln(2 + sqrt(3))
'rectform((1+i)^i)'                  e^(-pi/4)*cos(ln(2)/2) + i*e^(-pi/4)*sin(ln(2)/2)
```

### Areas, volumes and integrals over regions

`integrate(f, x, a, b, y, c, d)` integrates over several variables; the first range is the
outer one and the ends of an inner range may depend on the outer variables. Regions of the
plane are `rectangle`, `disk`, `annulus`, `sector`, `ellipse`, `triangle` and `polygon`, with
points written as `[x, y]` or as complex numbers; regions of any dimension are `box`, `ball`,
`shell`, `ellipsoid`, `simplex`, and in space also `cylinder` and `cone`.

`volume(f, z, region)` is the volume under the surface `f` over a region of the complex plane:
`re(f(z))`, `im(f(z))`, `abs(f(z))` and `arg(f(z))` are surfaces over the plane of `z`. With
`[x, y]` in place of `z` the integrand is a function of two real variables. A closed form is
given when one is found and confirmed numerically; otherwise the value is numeric, with the
digits that are certain.

```
'integrate(x*y, x, 0, 1, y, 0, x)'            1/8
                                                ~ 0.125
'integrate(e^(-x^2-y^2), x, -inf, inf, y, -inf, inf)'
                                              pi
                                                ~ 3.1415926535897932384626433832795
'integrate(abs(x-y), x, 0, 1, y, 0, 1)'       1/3
                                                ~ 0.33333333333333333333333333333333
'integrate(1/(1+x*y), x, 0, 1, y, 0, 1)'      pi^2/12
                                                ~ 0.82246703342411321823620758332301
'volume(re(z^2), z, disk(0, 1))'              0
'volume(abs(z)^2, z, disk(0, 2))'             8pi
                                                ~ 25.132741228718345907701147066236
'volume(im(e^z), z, rectangle(0, 1+pi*i))'    2e - 2
                                                ~ 3.4365636569180904707205749427053
'volume(arg(z), z, sector(0, 1, 0, pi/2))'    pi^2/16
                                                ~ 0.61685027506808491367715568749226
'volume(abs(im(z^2)), z, disk(0, 1))'         1
'volume(abs(sin(z)), z, disk(0, 1))'          2.100628697330130024451715148702
'volume(re(sin(z)), z, triangle(0, 1, i))'    -1/(4e) + e/4 - sin(1)/2
                                                ~ 0.16686510441795247511493976448265
'integrate(x^2+y^2+z^2+w^2, [x,y,z,w], ball([0,0,0,0], 1))'
                                              pi^2/3
                                                ~ 3.2898681336964528729448303332921
'area(polygon(0, 2, 2+2i, 1+i, 2i))'          3
'area(x^2, x+2, x)'                           9/2
                                                ~ 4.5
'area(sin(x), x, 0, 2pi)'                     4
'volume(ball([0,0,0,0,0], r))'                8pi^2*r^5/15
'surfacearea(cone([0,0,0], 3, 4))'            24pi
                                                ~ 75.398223686155037723103441198708
'centroid(sector(0, 1, 0, pi))'               [0, 4/(3pi)]
'arclength(x^2, x, 0, 1)'                     asinh(2)/4 + sqrt(5)/2
                                                ~ 1.4789428575445974338279060194339
'surfacearea(x^2+y^2, [x,y], disk(0,1))'      pi*(5sqrt(5) - 1)/6
                                                ~ 5.3304135002689731212198716387908
```

### Functional equations

`funsolve(equation, f)` looks for the functions that satisfy an equation: linear equations
whose arguments form a finite orbit (`x`, `1/x`; `x`, `1 - x`; ...), equations in `x + c`, `k x`
or `x^p` (through recurrences), and equations in two variables (through families of
polynomials, exponentials, logarithms and powers). Every answer is checked in the equation.
`C1`, `C2`, ... are constants.

```
'funsolve(f(x) + 2f(1/x) = 3x, f)'            f(x) = 2/x - x
'funsolve(f((x-1)/x) + f(x) = 1 + x)'         f(x) = (x^3 - x^2 - 1)/(2x*(x - 1))
'funsolve(f(x+y) = f(x) + f(y))'              f(x) = C1*x
'funsolve(f(x+y) = f(x)*f(y))'                f(x) = 0
                                              f(x) = e^(C1*x)
'funsolve(f(x*y) = f(x) + f(y))'              f(x) = C1*ln(x)
'funsolve(f(x+y) + f(x-y) = 2f(x)*f(y))'      f(x) = 0
                                              f(x) = cosh(C1*x)
'funsolve(f(x+y) = (f(x)+f(y))/(1-f(x)*f(y)))'
                                              f(x) = -i
                                              f(x) = i
                                              f(x) = tan(C1*x)
'funsolve(f(x+y) = f(x)*cos(y) + f(y)*cos(x))'
                                              f(x) = C1*sin(x)
'funsolve(f(x+1) = x*f(x))'                   f(x) = C1*gamma(x)
                                              note: the constants may also be functions of period 1
'funsolve(f(2x) = 4f(x))'                     f(x) = C1*x^2
                                              note: the constants may also be functions unchanged when x is multiplied by 2
```

### Iterating a function a fractional number of times

`f[t](x)` is the function `f` applied `t` times, for any function name; `iterate(expr, x, t)`
does it for the map `x -> expr`, and `iterate(expr, x, t, x0)` gives the value at `x0`. `t` may
be a whole number, a fraction, negative or complex: `f[0]` is the identity, `f[1]` is `f`,
`f[2](x)` is `f(f(x))`, `f[-1]` is the inverse function, and `f[s](f[t](x)) = f[s + t](x)`, so
`f[1/3]` applied three times is `f`.

Closed forms exist for `a x + b`, `c x^p`, `(a x + b)/(c x + d)`, for the polynomials up to the
degree 12 that a shift of `x` turns into a power or a Chebyshev polynomial (`x^2 - 2`,
`4x^3 - 3x`, `x^3 + 3x`, `(x + 1)^3 - 1`, ...), and for `b^x`, where the iterate is tetration:
`exp[1/2](x) = e^^(slog(e, x) + 1/2)`. A whole `t` is the map, or a formula for its inverse,
again and again.

```
'iterate(2x+1, x, 1/2)'                   sqrt(2)*x - 1 + sqrt(2)
'iterate(2x+1, x, t)'                     x*2^t + 2^t - 1
'iterate(x^2, x, 1/2)'                    x^(sqrt(2))
'iterate(x/(1+x), x, t)'                  x/(t*x + 1)
'iterate(4x*(1-x), x, t)'                 1/2 - cosh(2^t*acosh(1 - 2x))/2
'iterate(x^3+3x, x, t)'                   2sinh(3^t*asinh(x/2))
'iterate(8x^4-8x^2+1, x, 1/2)'            cosh(2acosh(x))
'exp[-1](x)' 'exp[2](x)' 'exp[1/2](x)'    ln(x)
                                          e^(e^x)
                                          e^^(slog(e, x) + 1/2)
'exp[0.(3)](exp[0.(3)](exp[0.(3)](x)))'   e^x
'iterate(x^3, x, i)'                      x^(3^i)
```

Any other map is iterated at a number, with all the digits asked for, at a fixed point of the
map. A note tells which fixed point it is and how the iterate is made:

- a fixed point that the orbit of the number runs into (a slope of a size below 1): regular
  iteration, where the map is `w -> slope * w` in the right coordinate;
- the slope 1 (`e^x - 1`, `sin(x)`, `x + x^2`): the same from the side that the map runs into,
  or from the side that it runs out of;
- the slope 0 (`x^2 + x^3`, `sin(x)^2`): Boettcher's coordinate, where the map is `w -> w^m`;
- infinity, for a quotient of polynomials whose orbit runs away: `x^2 + 1` has the slope 0
  there in `1/x`, and `iterate(x^2+1, x, 1/2, 1)` is a real number that the same iterate takes
  to 2;
- a slope of the size 1 that is no root of 1 (`e^(i)*x + x^2`): the map is a rotation in a disk
  around the point;
- else a fixed point that the map runs away from, which its inverse runs into (`2sinh(x)`,
  `x^2 - 1`, `x!`).

```
'cos[1/2](1)'                                 0.75308905103837029464247966234978 + 0.21095439744221607041226043321636i
                                              note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)
'cos[1/2](cos[1/2](1))' 'cos(1)'              0.54030230586813971740093660744298
                                              note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)
                                              cos(1)
                                                ~ 0.54030230586813971740093660744298
'sin[1/2](1)'                                 0.90870842974336355496561124039192
                                              note: iterate: regular iteration at the fixed point 0 (slope 1)
'iterate(e^x-1, x, 1/2, 1)'                   1.2710274138899515214246518766723
                                              note: iterate: regular iteration at the fixed point 0 (slope 1)
'iterate(x^2+1, x, 1/2, 1)'                   1.4122853525689035276835226679979
                                              note: iterate: Boettcher's coordinate at infinity (degree 2)
'abs(iterate(x^2+1, x, 1/2, iterate(x^2+1, x, 1/2, 1)) - 2) < 10^-30'
                                              true
                                              note: iterate: Boettcher's coordinate at infinity (degree 2)
'iterate(x^2+x^3, x, 1/3, 3/10)'              0.23732201663154615512401610376921
                                              note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)
'iterate(2sinh(x), x, i, 1)'                  0.70608783936644618903450771195459 + 0.64825832263855769458436854807591i
                                              note: iterate: regular iteration at the repelling fixed point 0 (slope 2)
'iterate(e^(i)*x+x^2, x, 1/2, 1/20)'          0.044836303794712618571262068267387 + 0.023126094046315493993809066424038i
                                              note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)
-d 60 'iterate(x!, x, 1/2, 4)'                6.70253073232877914069364378501774180550013486665146995853131
                                              note: iterate: regular iteration at the repelling fixed point 2 (slope 1.84556867)
```

`iterate(expr, x, t, x0, p)` takes the fixed point next to the number `p` (`inf` is infinity),
whatever the closed form. `sqrt(2)^x` has the fixed points 2 and 4, and its half iterates at
the two are not the same function:

```
'iterate(sqrt(2)^x, x, 1/2, 3, 2)'      2.9137867346334526079794417475975
                                        note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)
'iterate(sqrt(2)^x, x, 1/2, 3, 4)'      2.913786734633452607979441916978
                                        note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)
'iterate(e^x, x, 1/2, 1/2, 0.3+1.3i)'   0.99303919280011931894522795776518 + 0.13114284571242883021571603863683i
                                        note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)
'iterate(x^2-1, x, 1/2, -3/10)'         -0.88159462984144529854997076792026
                                        note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265
'iterate(x^2-1, x, 1/2, -3/10, -0.6)'   -0.67429403750566814940818095567885 + 0.41174675597706056375806695302735i
                                        note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)
```

Things to know about these numbers:

- The iterate is the one that goes on from the fixed point, and the powers of a slope and of
  Boettcher's coordinate are principal powers, as they are in the closed forms. `cos[1/2](1)`
  is not real because the slope of `cos` at its fixed point is negative.
- A map folds the line or the plane where its slope is 0. A point past a fold, as seen from
  the fixed point, gets the iterate of the point before the fold that the map takes to the
  same points, and the note names that point. Steps back that would go past a fold are an
  error, and so is a step that leaves the basin of a fixed point with the slope 0, or the
  disk of a rotation.
- `f[s](f[t](x)) = f[s + t](x)` holds for iterates at the same fixed point, up to what the
  notes say: a value past a fold, or a principal power that does not go along with the map,
  is not carried on by further iterates. With a `t` that is not real the value is a complex
  number whose own orbit may run elsewhere: give the fixed point then.
- The orbit decides. When it runs into a fixed point where there is no iterate (a slope -1,
  a point past the edge of a basin), the answer is an error that says so, and the fifth
  argument chooses another fixed point.
- A map with no fixed point (`x + e^x`) has no iterate of this kind.

```
'iterate(x/2+x^2/4, x, 1/2, -17/10)'    -0.1917013159269748671039126256453
                                        note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at -0.3, which the map takes to the same points
'iterate(x/2+x^2/4, x, -1/2, -1/2)'     error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far
'iterate(x^2+1, x, i, -2)'              error: the iterate leaves the basin of the fixed point with the slope 0: Boettcher's coordinate would have the size 1 or more
'iterate(sin(x)/2, x, 1/2, pi)'         0
                                        note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points
'iterate(x/2+x^2/4, x, -150, 10^-50)'   0.000014272273227338351618152345979664
'sin[1/2](x)'                           sin[1/2](x)
                                        note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point
```

### Tensors and curved spaces

A nested list is a tensor. `outer`, `contract`, `inner`, `transpose(t, order)`, `symmetrize`,
`antisymmetrize`, `wedge`, `hodge`, `kron` and `levicivita` work on tensors of any rank. For a
metric `g` in the coordinates `[x, y, ...]` there are `christoffel`, `riemann`, `ricci`,
`ricciscalar`, `einstein`, `kretschmann`, `geodesics`, `covariant`, `raiseindex`, `lowerindex`,
and `laplacian` and `divergence` with the metric as a third argument.

```
'outer([1,2],[x,y])'                          [[x, y], [2x, 2y]]
'contract(outer([a,b],[c,d]), 1, 2)'          a*c + b*d
'hodge([a,b,c])'                              [[0, c, -b], [-c, 0, a], [b, -a, 0]]
'christoffel([[1,0],[0,r^2]], [r,t])'         [[[0, 0], [0, -r]], [[0, 1/r], [1/r, 0]]]
'ricciscalar([[R^2,0],[0,R^2*sin(t)^2]], [t,p])'
                                              2/R^2
'kretschmann([[R^2,0],[0,R^2*sin(t)^2]], [t,p])'
                                              4/R^4
'geodesics([[1,0],[0,r^2]], [r,t])'           [r'' -> r*(t')^2, t'' -> -2r'*t'/r]
'laplacian(u^2, [u,v,w], [[1,0,0],[0,u^2,0],[0,0,u^2*sin(v)^2]])'
                                              6
'matrixexp([[0,1],[-1,0]], t)'                [[cos(t), sin(t)], [-sin(t), cos(t)]]
'matrixsqrt([[2,1],[1,2]])'                   [[1/2 + sqrt(3)/2, sqrt(3)/2 - 1/2], [sqrt(3)/2 - 1/2, 1/2 + sqrt(3)/2]]
```

### Geometry in any dimension

Points and vectors are lists of any length (or complex numbers in the plane). `line(p, q)` and
`plane(p, q, r)` are figures for `distance`, `angle`, `projection`, `intersection` and
`equation`. `polytope(name)` gives the numbers of a regular polytope with edge `a`.

```
'distance([1,2,3,4],[5,6,7,8])'               8
'angle([1,0,0,0],[1,1,1,1])'                  pi/3
                                                ~ 1.0471975511965977461542144610932
'angle([1,0],[0,0],[0,1])'                    pi/2
                                                ~ 1.5707963267948966192313216916398
'distance([0,0,5], plane([0,0,0],[1,0,0],[0,1,0]))'
                                              5
'intersection(line([0,0,0],[1,1,1]), plane([0,0,2],[0,0,1]))'
                                              [2, 2, 2]
'equation(plane([1,0,0],[0,1,0],[0,0,1]))'    x + y + z = 1
'solidangle([1,0,0],[0,1,0],[0,0,1])'         pi/2
                                                ~ 1.5707963267948966192313216916398
'cross([1,0,0,0],[0,1,0,0],[0,0,1,0])'        [0, 0, 0, 1]
'circumsphere([0,0,0],[1,0,0],[0,1,0],[0,0,1])'
                                              [[1/2, 1/2, 1/2], sqrt(3)/2]
'ballvolume(4, r)' 'spherearea(4, r)'         pi^2*r^4/2
                                              2pi^2*r^3
'rotationmatrix(pi/2, [1,0,0,0], [0,0,0,1])'
                                              [[0, 0, 0, -1], [0, 1, 0, 0], [0, 0, 1, 0], [1, 0, 0, 0]]
'polytope(tesseract)'                         [vertices -> 16, edges -> 32, faces -> 24, cells -> 8, volume -> a^4, surface -> 8a^3, circumradius -> a, inradius -> a/2, dihedral -> pi/2]
'polytope(cell24, 2)'                         [vertices -> 24, edges -> 96, faces -> 96, cells -> 24, volume -> 32, surface -> 64sqrt(2), circumradius -> 2, inradius -> sqrt(2), dihedral -> 2pi/3]
'polytope(simplex, 5)'                        [vertices -> 6, edges -> 15, facets -> 6, volume -> sqrt(3)*a^5/480, surface -> sqrt(5)*a^4/16, circumradius -> sqrt(15)*a/6, inradius -> sqrt(15)*a/30, dihedral -> acos(1/5)]
```

### Graphs

A graph is the list of its edges: `[a, b]` joins two vertices, `a -> b` goes one way, a third
entry is a weight (`[a, b, 5]`, `[a -> b, 5]`), and `[v]` is a vertex alone. Vertices are
numbers or names. `completegraph`, `cyclegraph`, `pathgraph`, `stargraph`, `wheelgraph`,
`gridgraph`, `hypercubegraph`, `petersengraph` and `completebipartite` make such lists.

```
'degrees([[1,2],[2,3],[3,1],[3,4]])'          [2, 2, 3, 1]
'adjacencymatrix([[1,2],[2,3],[3,1],[3,4]])'
                                              [[0, 1, 1, 0], [1, 0, 1, 0], [1, 1, 0, 1], [0, 0, 1, 0]]
'components([[1,2],[3,4],[5]])'               [[1, 2], [3, 4], [5]]
'shortestpath([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a, b)'
                                              [a, c, b]
'graphdistance([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a, b)'
                                              4
'spanningtree([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])'
                                              [[b, c, 1], [a, c, 3], [c, d, 2]]
'spanningtreecount(completegraph(5))'         125
'diameter(petersengraph())' 'girth(petersengraph())'
                                              2
                                              5
'chromaticpolynomial(cyclegraph(5))'          x^5 - 5x^4 + 10x^3 - 10x^2 + 4x
'chromaticnumber(petersengraph())'            3
'graphcoloring(wheelgraph(6))'                [[1], [2, 4], [3, 5], [6]]
'hamiltoniancycle(hypercubegraph(3))'         [0, 1, 3, 2, 6, 7, 5, 4, 0]
'hamiltoniancycle(petersengraph())'           []
'eulerianpath([[1,2],[2,3],[3,1],[3,4],[4,5],[5,3]])'
                                              [1, 2, 3, 4, 5, 3, 1]
'maxflow([[s -> a, 3], [s -> b, 2], [a -> b, 1], [a -> t, 2], [b -> t, 3]], s, t)'
                                              5
'maxmatching(petersengraph())'                [[1, 2], [3, 4], [5, 10], [6, 8], [7, 9]]
'maxclique(wheelgraph(6))' 'maxindependentset(petersengraph())'
                                              [1, 2, 3]
                                              [3, 5, 6, 7]
'topologicalsort([a -> b, b -> c, a -> c, d -> a])'
                                              [d, a, b, c]
'strongcomponents([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])'
                                              [[1, 2, 3], [4]]
'isisomorphic(petersengraph(), complementgraph(linegraph(completegraph(5))))'
                                              true
'pagerank([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 2])'
                                              [380/1769, 703/1769, 686/1769]
'eigenvalues(adjacencymatrix(cyclegraph(4)))'
                                              [-2, 0, 0, 2]
```

Distances are exact sums of the weights. The chromatic polynomial is found for graphs of up to
16 vertices; the searches for colorings, cliques, Hamiltonian ways and isomorphisms stop with
an error after 20 seconds.

### Units

Inside `convert`, `siunits`, `dimension` and `quantity` the names of units are units; elsewhere
`m`, `s` or `J` are symbols like any other. The SI units take the prefixes (`km`, `ms`, `GHz`,
`kWh`, `MeV`, `MiB`). `symbolic-math --units` lists the units and the prefixes.

```
'convert(100 km/h, m/s)'                      (250/9) m/s
                                                ~ 27.777777777777777777777777777778 m/s
'convert(5 ft + 3 in, cm)'                    160.02 cm
'convert(1 kWh, MJ)'                          3.6 MJ
'convert(72 degF, degC)' 'convert(0 K, degC)'
                                              (200/9) degC
                                                ~ 22.222222222222222222222222222222 degC
                                              -273.15 degC
'convert(100 min, [h, min])'                  [1 h, 40 min]
'convert(90 deg, rad)'                        (pi/2) rad
                                                ~ 1.5707963267948966192313216916398 rad
'convert(1 GiB, MB)'                          1073.741824 MB
'siunits(60 mph)' 'siunits(1 N*m)'            26.8224 m/s
                                              1 kg*m^2/s^2
'dimension(5 kWh)'                            length^2*mass/time^2
'convert(planck*c/(500 nm), eV)'              (6621486190496429/2670294390000000) eV
                                                ~ 2.4796839686640052447550548911575 eV
'convert(1 mol*Rgas*300 K/(1 atm), l)'        (207861565453831/8443750000000) l
                                                ~ 24.617209824287905255366395262768 l
'convert(1 u, MeV/c^2)'                       931.494103719 MeV/c^2
'convert(5 km, m)/convert(2 h, s)'            (25/36) m/s
                                                ~ 0.69444444444444444444444444444444 m/s
'convert(x km, m)'                            (1000x) m
                                              note: x is taken as a number, not as a unit
'convert(5 m, s)'                             error: 5 m is length, s is time
```

Conversions are exact (`1 mi` is exactly `1.609344 km`); the measured constants of nature
(`G`, `me`, `mp`, `u`, ...) have the digits that are known (CODATA 2022). A number of `degC`
or `degF` alone is a point on that scale, in a product with other units a degree is a step.

### Plots

`plot(f, x, a, b)` draws a curve with Braille dots; `plot([f, g], x, a, b)` draws several.
After the range come, in any order, `[low, high]` for the vertical range, a width and a height
in characters, and `ascii` for plain marks. `parametricplot`, `polarplot`, `implicitplot`,
`listplot` and `histogram` draw curves of points, curves given by an equation, points and
counts.

```
$ symbolic-math 'plot(sin(x), x, 0, 2pi)'
 1 │           ⣀⡤⠒⠋⠉⠉⠙⠒⠦⣄
   │         ⡠⠞⠁        ⠈⠙⢦⡀
   │       ⣠⠎              ⠙⢦
   │     ⢀⠞⠁                 ⠱⣄
   │    ⡰⠋                    ⠈⢦⡀
   │  ⢀⡜⠁                       ⠳⡄
   │ ⢠⠎                          ⠘⢆
   │⡰⠃                            ⠈⢧⡀
 0 │⠁───────────────────────────────⢳⡀────────────────────────────⢠⠎
   │                                 ⠱⡄                          ⡰⠃
   │                                  ⠘⢦                       ⢀⡜⠁
   │                                   ⠈⠳⡀                    ⣠⠎
   │                                     ⠙⢆                 ⢀⡴⠁
   │                                       ⠳⣄              ⡰⠋
   │                                        ⠈⠳⣄⡀        ⢀⡴⠊
-1 │                                           ⠙⠲⠤⣄⣀⣀⣠⠤⠚⠉
   └────────────────────────────────────────────────────────────────
    0                                                            2pi
$ symbolic-math 'plot([sin(x), cos(x)], x, 0, 2pi, ascii, 60, 12)'
 1 |+++++      *********                                   +++++
   |    ++++****       ****                             ++++
   |      *+++            ***                         +++
   |    **   +++            ***                     +++
   |  **       +++            ***                 +++
   |**           +++            **              +++
 0 |*--------------++-------------**-----------++-------------**
   |                +++            ***       +++            **
   |                  +++            ***   +++            **
   |                    +++            **+++           ***
   |                       +++        +++****       ****
-1 |                          ++++++++      *********
   +------------------------------------------------------------
    0                                                        2pi
* sin(x)   + cos(x)
$ symbolic-math 'plot(tan(x), x, -3, 3, [-5, 5], 60, 12)'
 5 │            ⡇                 │            ⡜
   │           ⢰⠁                 │           ⢀⠇
   │          ⢀⠎                  │          ⢀⡞
   │         ⣠⠎                   │         ⣠⠞
   │     ⢀⣠⠴⠚⠁                    │      ⣀⡤⠚⠁
   │⣠⠤⠴⠒⠊⠉                        ⣀⣀⠤⠤⠒⠚⠉
 0 │───────────────────────⣀⡤⠤⠒⠒⠉⠉⠁───────────────────────⣀⡠⠤⠖⠒⠋
   │                   ⢀⡤⠚⠉       │                   ⢀⡤⠖⠋⠁
   │                  ⡴⠋          │                  ⡰⠋
   │                 ⡼⠁           │                 ⡰⠁
   │                ⢰⠁            │                ⢀⠇
-5 │                ⡜             │                ⢸
   └────────────────────────────────────────────────────────────
    -3                                                         3
$ symbolic-math 'polarplot(cos(3t), t, 0, pi, 40, 10)'
 0.9241 │           ⢠⠖⠲⢤⡀ │
        │           ⢸   ⠱⡄│
        │            ⢧⡀  ⢹⡀
        │             ⠱⣄  ⣇
        │              ⠈⠑⢦⣸⣠⠴⠒⠋⠉⠉⠉⠉⠉⠲⡄
      0 │──────────────⢀⡠⠞⢹⠙⠲⠤⣄⣀⣀⣀⣀⣀⠴⠃───────────
        │             ⡰⠋  ⡏
        │            ⡞⠁  ⣸⠁
        │           ⢸   ⡰⠃│
-0.9241 │           ⠘⠦⠴⠚⠁ │
        └────────────────────────────────────────
         -1.629                             2.067
$ symbolic-math 'implicitplot(x^2 + y^2 = 1, x, -1.5, 1.5, y, -1.5, 1.5)'
 1.5 │                │
     │                │
     │            ⢀⣀⣀⣠⣄⣀⣀⡀
     │         ⣠⠴⠚⠉   │  ⠉⠓⠦⣄
     │       ⣠⠞⠁      │     ⠈⠳⣄
     │      ⣰⠃        │       ⠘⣆
     │     ⢰⠃         │        ⠘⡆
     │     ⣸          │         ⣇
   0 │─────⢹──────────┼─────────⡏─────
     │     ⠸⡄         │        ⢠⠇
     │      ⠹⡄        │       ⢠⠏
     │       ⠙⢦⡀      │     ⢀⡴⠋
     │         ⠙⠲⢤⣀   │  ⣀⡤⠖⠋
     │            ⠈⠉⠉⠙⠋⠉⠉⠁
     │                │
-1.5 │                │
     └────────────────────────────────
      -1.5                         1.5
$ symbolic-math 'listplot([1, 4, 9, 16, 25, 36], 40, 8)'
36 │                                       ⠙
   │
   │                               ⢴⠄
   │
   │                       ⢴⠄
   │                ⡀
   │        ⡀      ⠈⠋
 1 │⣄      ⠈⠋
   └────────────────────────────────────────
    1                                      6
$ symbolic-math 'histogram([1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6, 7])'
  1 .. 2.2 │████████████████████████████████████ 3
2.2 .. 3.4 │████████████████████████████████████ 3
3.4 .. 4.6 │████████████████████████████████████████████████ 4
4.6 .. 5.8 │████████████████████████████████████ 3
5.8 .. 7   │████████████████████████████████████ 3
```

### Numbers in a base

A number can be written in any base from 2 on: `base#digits`, with a point and with the digits
that repeat in parentheses. The digits are `0-9` and `a-z` up to base 36 (capitals are the same
digits there), then `A-Z`, `@` and `_` up to base 64, as in the shell. From base 65 on every
digit is a number after a `#`, padded with zeros: `1000#001#234` is 1234. `0x`, `0b` and `0o`
are the bases 16, 2 and 8.

```
'16#ff'                   255
'2#101.1'                 11/2
                            ~ 5.5
'2#0.(01)'                1/3
                            ~ 0.33333333333333333333333333333333
'64#Az@_'                 9584575
'1000#001#234'            1234
'1000#000.(#333)'         1/3
                            ~ 0.33333333333333333333333333333333
'0x1F.8 + 0b0.1 + 0o17'   47
```

`base_conv(x, from, to)` takes `x` as it is written in the base `from` (no quotes are needed)
and writes it in the base `to`; `base_conv(x, to)` takes a value, which has no base until it is
written: a constant, an expression, a list. The answer shows the digits alone; with the base
and `#` in front it is a number that can be typed again. A fourth argument (or the third of
`tobase`) is the number of digits of a number without an end.

```
'base_conv(15, 10, 16)'               f
'base_conv(fe, 16, 10)'               254
'base_conv(64#Hello, 64, 10)'         725177688
'base_conv(ff.8, 16, 2)'              11111111.1
'base_conv(0.(3), 10, 3)'             0.1
'base_conv(11/101, 2, 10)'            0.6
'base_conv(pi, 16)'                   3.243f6a8885a308d313198a2e03...
'base_conv(e, 2)'                     10.101101111110000101010001011000101000101011101101001010100110101010111111011100010101100010000000100111001...
'base_conv(pi, 10, 16, 50)'           3.243f6a8885a308d313198a2e03707344a4093822299f31d00...
'base_conv(1234, 10, 1000)'           #001#234
'base_conv(#001#234, 1000, 10)'       1234
'base_conv(1/3, 10, 1000)'            #000.(#333)
'base_conv(2^64, 10, 64)'             g0000000000
'base_conv(Hello, 256, 10)'           310939249775
                                      note: "Hello" is read character by character: #072#101#108#108#111
'base_conv(310939249775, 10, 256)'    #072#101#108#108#111
                                      note: as a text, one character for each digit: "Hello"
'base_conv([10, 255, 1/3], 10, 16)'   [a, ff, 0.(5)]
```

With a source base, a first argument that is no number of that base is an expression whose
numbers are written in that base (a number that names its base, like `64#Hello`, is the same
in every base): `base_conv(101 + 11, 2, 10)` is 8, and `base_conv(pi, 10, 16)`
is pi. A word of digits is a number first: `base_conv(e, 16, 2)` is 14 in base 2, and
`base_conv(e, 10, 2)` is Euler's number. In base 256 a text without `#` is read character by
character, and an answer whose digits are all characters is also shown as a text.

`tobase(x, b)` writes the digits, `tofraction(x, b)` the fraction with numerator and
denominator in the base, `digits(x, b)` lists the digits (of the whole part, after the point,
and those that repeat), `frombase` reads such lists and texts, `period(x, b)` is the number of
digits that repeat, and `digitcounts(x, b, n)` counts how often each digit stands among the
first `n`. An answer in a base is a number like any other: `tobase(1/3, 2) + 1` is `4/3`.

```
'tobase(1/7)'                        0.(142857)
'tobase(1/3, 2)'                     0.(01)
'tobase(pi, 2, 40)'                  11.00100100001111110110101010001000100001...
'tobase(1/3 + i/7, 2)'               0.(01) + 0.(001)*i
'tofraction(2#0.(01), 2)'            1/11
'tofraction(0.75, 16)'               3/4
'digits(1/7)'                        [[0], [], [1, 4, 2, 8, 5, 7]]
'digits(pi, 60, 5)'                  [[3], [8, 29, 44, 0]]
'frombase([[3], [8, 29, 44]], 60)'   84823/27000
                                       ~ 3.1415925925925925925925925925926
'frombase("ff.8", 16)'               511/2
                                       ~ 255.5
'period(1/97)'                       96
'digitcounts(pi, 10, 1000)'          [93, 116, 103, 103, 93, 97, 94, 95, 101, 105]
```

### Walks along the digits of a number

`walk(x, base, digits)` takes a step of length 1 for every digit of a real number, the whole
part first: the digit `d` goes in the direction of `360*d/base` degrees, counted against the
clock from the x-axis. The base is 10 and the number of digits 1000 when they are left out; a
number whose digits end gives a walk that ends, and a fraction walks its block of digits again
and again. As a statement the walk is drawn with characters (`width = 64`, `height = 20` and
`ascii` change the picture), with what there is to tell about it.

```
$ symbolic-math 'walk(pi, 10, 1000)'
 28.25 │                  │  ⢠⣠⣀  ⢀⡀                     ⢀⡄
       │                  │  ⠉⠁⡏⣹⣰⠙⢅           ⢀⡠⡀      ⢀⡼⡃
       │                  │  ⢠⢬⠛⣹⣒⣤⣼⣧⣷⣾⣶⠁    ⡠⣴⣉⡀⢣⢴⡤⠤⣖⣲⣿⡿⡾
       │                  │⢀⣀⡘⠾⢿⣿⣵⣶⣿⣿⣟⣿⣿⡦    ⠈⣉⣷⣏⣱⣷⠿⠒⡗⢻⣿⡽⠋
       │                ⣀⣠⢒⠾⣿⣦⣤⣺⣯⢿⣿⣿⣮⣣⣿⣷⣏  ⢀  ⢀⢨⠂⢟⣿⣀⡀⢱⣸⣿⠗
       │                ⢺⣿⣿⣿⣿⣿⣿⣿⣽⠛⠊  ⠡⠟⠋⢉⣵⡄ ⠉⣾⣟⠋ ⠈⡱⠘⠃
       │                 ⠘⠛⣿⢿⣻⣿⡊⠉⠁       ⠠⠧⡤⠊⡏⢀⢑⠢⣠⠧⡄⡀
       │             ⢸⠉⢲⣠ ⠼⢬⣿⣿⣿⣞⠆         ⠐⠁ ⢸⡽⣽⣤⣮⣿⣶⣵  ⢀⣀⣠⢤⡀
       │            ⣤⢼⣶⣷⣣⠚⣦⣤⣭⣟⣿⠜            ⢀⡏⠟⠊  ⠈⣿⣿⣭⣿⣽⣤⡤ ⠱
       │            ⢸⣴⣿⠏⣵⣤⣿⡽⡿⡿⠃              ⠓⠒  ⠠⠊⠁⠉⠙⡏⠻⠿⠃
       │            ⠈⢮⡧⣩⠃⠘⠛⠈⠁
       │            ⠘⠗⠋⢪  │ ⢠      ⣠
       │                ⠙⣺⣶⣄⣯⣦⣄⣠⡴⣧⣠⣿⣶⢶⡤⣄
       │                ⠈⢈⣯⣿⡟⠟⠁⠈⠉⠊⠟⣟⠷⣹⣯⠟⠉⠁
       │            ⢠ ⡀   │⠹     ⣠⠴⠿⠭⠺⠾
       │            ⢼⢾⣷⢤  │ ⢀⣀⡀⣴⣖⠟⠿
       │             ⠣⡬⣾⢿⣼⣽⣄⣠⠝⢿⣿⠁⣤⡤
       │               ⠉⣽⢫⡿⠿⣧⡀⢿⢿⣿⣝
     0 │────────────────⠈⠃⠇──⢙⣿⣻⣝⡭⠂─────────────────────────────────────
-1.981 │                  │  ⠚⠛⠓
       └────────────────────────────────────────────────────────────────
        -13.83                                                     34.54
1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees
from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53
$ symbolic-math 'walk(1/7, 10, 60, width = 40, height = 10)'
  4.251 │      ⣆  ⡀                  │
        │     ⡸⠘⡄⢠⠻⡀ ⣴  ⢀            │
        │   ⠒⠒⢗⣒⣛⣜⣀⣣⢰⠁⢇ ⡎⡆ ⢰⡄  ⡀     │
        │     ⢠⠋⠒⣜⡶⣒⠺⣒⣚⣸⣀⣸⣀⠇⢣ ⡸⢇ ⢠⡆  ⡀
        │     ⣎⠔⠊⢸⢀⡠⢓⠇⠓⣌⡲⢖⡺⣒⠚⢢⠃⠘⡄⡜⠸⡀⢰⢱  ⣦  ⢀
        │    ⠈   ⠗⠁ ⣸⡠⠊⢠⠃⣀⠜⡜⠙⢫⣹⢭⡵⢥⡤⠧⡇│⢇⡸⠈⡆⢀⠟⡄
        │           ⠁  ⠞⠊ ⢰⡡⠒⠁⡎⣀⠜⢺⠉⢫⣙⠯⣭⢧⠤⠵⡜ ⢣
        │                 ⠁  ⠸⠚ ⢀⣇⠔⠃⡜⢀⡠⢳⠋⠛⣍⠫⣉
        │                       ⠈  ⠠⠓⠁ ⣎⠔⠊ ⢀⡠⠓
-0.1037 │────────────────────────────⠠⠤⠥⠤⠤⠔⠊─────
        └────────────────────────────────────────
         -6.251                             2.459
60 steps along the digits 0.14285714285714285714285... of 1/7 in base 10: the digit d is a step at 36*d degrees
from the origin to (-5.601, 3.196), 6.449 away; x from -5.601 to 1.809, y from 0 to 4.147
$ symbolic-math 'walk(1/4, 4, 10, width = 24, height = 5, ascii)'
 1.025 |       |        *
       |       |        *
       |       |        *
       |       |        *
-0.025 |-------**********-------
       +------------------------
        -0.76               1.76
2 steps along the digits 0.1 of 1/4 in base 4, which end there: the digit d is a step at 90*d degrees
from the origin to (1, 1), 1.414 away; x from 0 to 1, y from 0 to 1
```

`walk3d(z, base, digits)` walks in space along a complex number: at every place the digit `a` of
the real part turns the step by `360*a/base` degrees around the z-axis and the digit `c` of the
imaginary part lifts it by `360*c/base` degrees above the plane, so that the step is
`(cos(c)*cos(a), cos(c)*sin(a), sin(c))`. A real number walks in the plane as `walk` does.

```
$ symbolic-math 'walk3d(e + pi*i, 12, 2000, width = 60, height = 16)'
                              ⣆
                           ⢺⡄⣤⣿⡦
                          ⣀⡞⣻⣾⣾⣯⣤⣤⣀
                     ⢠⣀⣴⣯⠙⣷⣿⣿⣻⣿⣿⣿⣿⡟⢶
                     ⠈⣿⣿⣏⣹⢎⠽⣻⣿⣿⣻⣿⣿⢷⣎
                      ⠈⠁⠸⣿⣷⣾⢼⣿⣿⣿⣿⣿⣯⣛⡀
                        ⢰⡖⢺⣿⣼⣽⣽⣿⣿⣿⠍⠙⡇
                         ⠸⠿⡻⣿⣿⣿⣽⣿⢿⣆⣄⣉⠆
                           ⢸⣿⣿⢻⣿⣿⣾⣿⠻⠁
                          ⢀⣈⢻⠸⠿⣿⣿⣿⣽⣄⡄
                          ⠘⣟⣾⣿⣭⢻⠿⠯⡛⣾⣡⣖
                          ⠲⠧⢭⢸⠁⡀ ⡤⣽⡟⠈⣯⡀
                            ⢼⢸⣾⣿⣿⣗⣤⣾⣟⣿⣆
                           ⣶⡗⠷⠦⣿⣿⣇⠈⣿⡯⠙⠁
                          ⠈⣯⣧⡔⠛⢻⣿⡃ ⠸
                           ⠘⢻⡇  ⠛⠻
2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane
from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53
```

### Walks along the primes

`primewalk(n, m, k)` takes a step for every whole number from 2 to the `n`-th prime and turns
left by `k/m` of a full turn at every prime (`m = 4` and `k = 1` when they are left out), so
that its sides are as long as the gaps between the primes. `primewalk3d(n, m)` does the same
in space: at every prime it takes the next of `m` directions: 3 (x, y, z), 4 (to the corners of
a tetrahedron) or 6 (x, y, z, -x, -y, -z). `primemodwalk(n, m)` takes one step for each of the
first `n` primes `p`, in the direction of `360*(p mod m)/m` degrees (`m = 8`: the four odd
remainders balance; a modulus without a square factor, like 10, has a direction that it
prefers), and `primemodwalk3d(n, m, k)` uses the remainders by `m` and by `k` as the two angles
of a step in space. `ulamspiral(n)` and `sacksspiral(n)` mark the first `n` primes on the
square spiral of the whole numbers and on the spiral where `k` is `sqrt(k)` away from the
middle after `sqrt(k)` turns. `primes_walk`, `primes3d_xyz` and other spellings are accepted.

```
$ symbolic-math 'primewalk(2000, width = 60, height = 16)'
547.5 │                          │ ⣤⣤⣄⣤⣠⡀
      │                          │ ⠉⠻⠽⣿⣻⡇
      │                          │  ⢠⡖⠻⠏⣹⣿⠃
      │                         ⢠⣤⣤⣿⠏⠁⣀⣾⣿⠛⠿⠇
      │                         ⠈⠛⢻⣷⣿⣿⣿⣿⣿⣠⣶⡤⡄
      │                        ⢀⣶⠛⠙⠙⣿⡯⠘⠛⡿⢯⠙⠒⣿
      │                        ⠸⣶⡶⠤⠶⡏   ⢁⣭⣧ ⠉
      │                       ⠰⣾⣿⣿⣶⣄⠓⠛⡿⣾⣿⣿⠁
      │                       ⢸⣿⣷│⢨⡏  ⠙⠻⠿⠁
      │                       ⠘⣿⣯⣁⣶⠃
      │                        ⣙⣯⣿⡅
      │                      ⣀⣠⣼⣍⣉
      │                      ⠻⠿⠏⣿⡿⠷
      │                       ⢀⣤⣏⡿⠟⣿⠃
      │                        ⠿⣿⡇
-19.5 │──────────────────────────⠗─────────────────────────────────
      └────────────────────────────────────────────────────────────
       -469.6                                                 593.6
1999 sides along the first 2000 primes, from 2 to 17389: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes
from the origin to (-19, 238), 238.8 away; x from -77 to 201, y from -6 to 534
$ symbolic-math 'primemodwalk(5000, 12, width = 60, height = 14)'
 3.404 │                    ⣠⣦⣄⣀⣀⡀                           ⢀⣀⣀⣀⡀
       │                    ⣀⣅⣐⣭⣒⠯⢖⣠⣀            ⢀ ⢀ ⢀⣀⣔⣪⢽⣺⢭⣒⣭⣒⣭⣒⡽
     0 │────────────────────⠈⠛⠻⣛⠣⣖⡥⢝⡻⢄⡠⢄⡀───⣒⠦⣐⠦⣐⡧⣗⡧⢟⡻⢟⡻⢟⣺⢼⣚⠿⣛⠿⣛⠻⣓──
       │                      ⣠⠴⣏⡗⣏⡷⢬⡷⢮⡥⢶⣱⢒⣱⠶⣿⠶⣿⠖⣏⡗⣍⡷⢾⡷⢾⡷⢺⣹⠲⣿⠶⣿⠶⣏│
       │                  ⢀⢄⣲⣭⣶⣭⡖⡯⠗⣮⣵⣮⣵⣮⣵⢺⢽⢲⣭⣶⣭⣶⡭⡗⡯⡗⣮⣵⣮⣵⣮⢽⢺⢭⣶⡭⠶⠭⠖⠉
       │                 ⣠⢭⣚⠭⣛⠭⣛⠭⣗⡯⢝⡫⠝⡫⢍⡫⢽⣺⠭⣛⠭⣛⠭⣓⡯⣗⡯⢝⡫⢝⡫⢝⣺⢽⣚⠭⣃⠄  │
       │                ⣠⢼⣹⠼⣿⠿⣿⠿⣏⡧⣏⡿⢿⡶⢶⡿⢿⣹⢼⣩⠽⣿⠿⣿⠧⣏⡧⣏⡿⢿⡿⢿⡿⢼⡙⠜⠋⠉⠁  │
       │                ⠈⣩⢺⣹⣶⣿⣶⣿⠖⢏⠗⣾⣷⣾⣷⣾⣷⢺⣹⢲⣿⣶⣿⣶⣏⡗⣍⡗⣾⣁⠈⣷⠮⡙⠂      │
       │       ⢠⣒⡯⣗⠪⣔⣢⣕⣪⢵⣺⢽⣒⣭⣒⣭⣒⠤⣀⠈⢐⠪⣕⣪⣕⣢⢽⣺⢭⣒⣥⣒⣭⣒⡯⣗⡮⣕⣪⣕⡊⠁        │
       │       ⣘⠁⣑⡧⢖⡻⢟⡻⢟⣺⢼⣺⠼⣛⠿⣛⠽⣓⡥⣖⡲⢟⡻⢟⡫⢍⣺⢼⣚⠿⣃⠼⣛⠧⣕⡡⠓⠉⠙⠉          │
       │     ⠰⣾⠦⣉⠧⣋⠷⢾⡓⢾⡷⢼⣹⢼⣩⠤⣵⠶⣯⠤⣏⡧⢀ ⠈⠁ ⠁ ⠉⠴⡏⠴⣿⠶⣎⡥⣏              │
       │  ⠢⢤⡖⠉ ⠉ ⠈⠓⣪⣑⣮⣵⣮⢽⠺⢽⢲⣭⣶⣭⣶⡭⡕⠉⠑⠤⣤⣦⣤⣊⠝⠚⠉⠲⠭⠴⠭⠔⠯⠁              │
       │    ⠉⠙⠭⠒⠤⠖⠯ ⠈⠉⠂⠁ ⠉⠺⠭⣛⠭⠛⠭⠑⠩⣗ ⢐⡫⢕⡫⠅                        │
-19.17 │                         ⠈⠣⠉⠲⠛⠉⠉⠁                        │
       └────────────────────────────────────────────────────────────
        -46.2                                                  2.171
5000 steps, one for every prime p up to 48611: at 30*(p mod 12) degrees
from the origin to (-27.21, -11.13), 29.4 away; x from -44.53 to 0.5, y from -18.63 to 2.866
$ symbolic-math 'ulamspiral(2000, width = 50, height = 16)'
 69.8 │          ⡄⡀⠠⢀⢄⣄⠠ ⡄⠄⢠ ⣀⠄⡠⡀⡤⡠⢀⡄⡠⡀⠄⠠⡄⢀⠄⡀⠠⢠
      │          ⢺⡙⡃⡎⡋⠷⠖⠅⢭⣰⠸⠻⢂⢏⡽⢏⣊⠅⡩⡣⡜⠡⣖⢪⠍⠩⠨⠩⡮⠩
      │          ⡳⢻⠶ ⣔⡠⡂⣵⡅⡑⠠⢯⣄⢨⣂⢂⠷⠯⢐⠖⠋⢢⡟⢅⡷⣂⠲⡇⢖⡊
      │          ⠤⠦⡱⠿⢁⠊⡰⢄⡮⢊⢴⠃⢦⢶⣥⣯⢣⠃⠣⢯⡗⣕⡧⡰⣴⣓⣄⢎⠣⣷
      │          ⡃⢄⢲⡪⢹⣜⢗⣢⣓⣪⢠⢒⣥⡎⢪⢀⡙⣤⢶⡠⣅⢪⠌⡦⣥⡨⠢⢓⢮⠨
      │          ⠞⠦⢂⠞⠖⣒⠒⡜⢖⡂⡱⣎⠪⡙⣜⣭⠺⡖⣎⢩⢤⠵⡹⢎⡈⠑⣬⠒⠡⠑
      │          ⠩⢏⢍⣇⡁⢐⡮⡩⠫⢁⢾⡨⢭⢨⠚⡷⣶⡏⢚⡍⣓⠚⠏⡢⠑⢈⠒⠓⣒⠠
      │          ⡔⢐⢢⡝ ⠲⡳⣁⣗⠪⡝⣗⣣⣳⣿⣮⠟⡻⡷⣛⠖⡛⡶⠔⠊⡠⢗⢧⠅⡡
    0 │──────────⠮⢁⡍⣃⠪⢄⣬⡭⠏⠜⢧⡊⣖⣼⢿⡯⡫⢎⠿⡼⠰⢎⢽⠡⠦⢲⠡⡿⠘⢼──────────
      │          ⡔⡌⠛⣕⣁⣫⣞⠬⡮⡧⣐⡝⣛⣼⠹⡵⣋⠥⣩⡏⢋⡲⣪⣘⢏⣡⠊⢇⡠⣩
      │          ⠣⡡⢑⢯⢐⢍⡐⢃⢭⡕⣈⠏⠨⢴⢡⢒⡺⢁⣬⢣⠋⢘ ⢡⣳⢈⡧⠃⣉⢛
      │          ⣍⠁⠨⡍⢍⠫⣡⡼⠩⡀⠾⡕⢡⢌⠚⡉⡿⠊⠓⠏⣬⡪⡓⡑⡴⣺⣑⣝⣁⢃
      │          ⠒⡐⠕⣌⠢⢶⡁⢛⢆⡖⢽⠎⢓⢗⡣⢚⠳⡑⡏⢢⡤⣪⣷⡂⠺⠇⠦⠋⡂⡞
      │          ⡁⢭⡂⠵⢙⠴⢎⡤⣚⡕⠤⡣⠼⠚⢭⡁⣣⠤⡤⡖⠳⢹⠳⢵⢕⠈⡥⡬⡨⣫
      │          ⡪⠔⡲⣉⢌⠳⡁⢄⡘⡂⣥⡞⠴⡨⠛⠼⣱⣊⡇⠄⡒⠑⡐⢡⡑⣨⢮⣨⡇⣹
-68.8 │          ⠃⠑⠂⠑ ⠂⠈⠁⠂⠊⠐⠂⠙⠘⠃⠙⠁⠂ ⠙⠘⠚ ⠉⠊⠁ ⠚⠈⠂
      └──────────────────────────────────────────────────
       -107.8                                       108.8
the first 2000 primes up to 17389 on the square spiral of the whole numbers: 1 in the middle, 2 to its right, then around against the clock
```

### Pictures of functions, graphs and matrices

Besides the plots above there are `complexplot(f, z, a, b)` (the values of a complex function
in the box with the corners `a` and `b`: the colour is the argument, the lightness the absolute
value, so that zeros are dark and poles light), `densityplot(f, x, a, b, y, c, d)` (the values
as the colours of a palette), `plot3d(f, x, a, b, y, c, d)` (a surface),
`parametricplot3d([x(t), y(t), z(t)], t, a, b)` (a curve of space), `graphplot(g)` (a graph
with its vertices on a circle) and `matrixplot(m)` (the entries of a matrix as colours). As
statements they are drawn with characters.

```
$ symbolic-math 'plot3d(sin(x)*cos(y), x, -pi, pi, y, -pi, pi, width = 60, height = 16)'
                            ⢀⠤⡲⠲⢄
                           ⣠⠟⢲⠣⣄⡸⢣
                         ⢀⡼⡟ ⡎ ⣰⠣⡀⠳⡀
                        ⢠⡿⢵⠓⢺⠤⣀⠇ ⠱⡜⢣
                      ⢀⡴⡟⠁⡇ ⡎ ⡸⠑⢄⣎⡱⣒⢷⢦⡀
               ⣤⣤⠶⣄⡀⡠⠖⠳⡼⢥⣰⣁⢰⠁⢀⠇ ⣸⢾⣞⢯⣮⣦⣿⣳⡄
             ⢠⢫⠚⠘⣄⣜⣿⢪⡢⣠⠃ ⡎ ⡝⠉⡺⠒⣴⡿⣩⣿⣏⢫⢿⡌⡷⣹⣆
            ⡰⡩⣋⠶⡏⢾⡵⡸⣯⢳⣗⢄⢰⠁⢀⠇ ⣇⣾⣷⠟⢳⠧⣉⣯⣻⡷⣼⢿⢽⣷⣄   ⣠⢲⠁
           ⢰⡱⠋⠁⠉⠙⢾⣿⢵⣵⢯⣿⡲⡗⠢⣼⣀⣼⣾⡟⡎ ⡜ ⢰⠑⢜⣿⡱⣧⢫⢿⡌⣯⡕⡫⢣⠃
           ⠁      ⠙⣿⢫⡆⢿⡽⡙⣷⡷⢫⣷⢏⣠⡃⢀⠇ ⡇ ⡰⠣⡻⣹⣖⠟⢎⢁⡸⡹⠁
                   ⠈⢷⣻⡾⡿⡿⣝⣿⣽⠃ ⡜⠈⢹⠑⢺⠲⢤⢧⣀⡬⠊⠛⢦⡼⠾⠎⠁
                     ⠉⠳⣼⣜⣫⠚⡞⢄⢠⠃ ⡇ ⡇⢀⣯⠖⠁
                        ⠱⣀⢯  ⡝⠢⢼⣀⣰⢥⡿⠁
                         ⢫⡀⠣⣰⠃ ⡎⢀⣧⠟⠁
                          ⠱⣠⠓⠦⣸⣀⡼⠋
                           ⠙⢤⣠⡣⠜⠁
the surface z = cos(y)*sin(x) over 24 by 24 cells; z from -1 to 1
$ symbolic-math 'densityplot(sin(x)*cos(y), x, -pi, pi, y, -pi, pi, width = 48, height = 8)'
 3.142 │⠶⡶⡶⣶⣶⣶⣾⣾⣾⣾⣿⣿⣿⣿⣾⣾⣾⣾⣶⣶⣶⡶⡶⠶⠶⠴⠴⠤⠤⠤⠄⠄⠄⠄    ⠄⠄⠄⠄⠤⠤⠤⠴⠴⠶
       │⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⠶⠶⠶⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶
       │⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⠶⠶⠶
       │⠶⠴⠴⠤⠤⠤⠄⠄⠄⠄    ⠄⠄⠄⠄⠤⠤⠤⠴⠴⠶⠶⡶⡶⣶⣶⣶⣾⣾⣾⣾⣿⣿⣿⣿⣾⣾⣾⣾⣶⣶⣶⡶⡶⠶
       │⠶⠴⠴⠤⠤⠤⠄⠄⠄⠄    ⠄⠄⠄⠄⠤⠤⠤⠴⠴⠶⠶⡶⡶⣶⣶⣶⣾⣾⣾⣾⣿⣿⣿⣿⣾⣾⣾⣾⣶⣶⣶⡶⡶⠶
       │⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⠶⠶⠶
       │⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⠶⠶⠶⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶
-3.142 │⠶⡶⡶⣶⣶⣶⣾⣾⣾⣾⣿⣿⣿⣿⣾⣾⣾⣾⣶⣶⣶⡶⡶⠶⠶⠴⠴⠤⠤⠤⠄⠄⠄⠄    ⠄⠄⠄⠄⠤⠤⠤⠴⠴⠶
       └────────────────────────────────────────────────
        -3.142                                     3.142
the colours of the palette run from the value -1 to 1
$ symbolic-math 'graphplot(cyclegraph(5), width = 40, height = 12)'
  1.212 │                    │
        │                  ⣀⠔⠷⣀
        │               ⢀⠔⠊  │ ⠑⠢⡀
        │            ⢀⠤⠊⠁    │   ⠈⠑⠤⡀
        │         ⣠⡠⠊⠁       │      ⠈⠑⢄⣄
        │         ⠈⡆         │        ⢰⠁
      0 │──────────⠸⡀────────┼───────⢀⠇──────────
        │           ⢣        │       ⡜
        │           ⠈⡆       │      ⢰⠁
        │            ⠸⡀      │     ⢀⠇
        │             ⣣⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣜
-0.9909 │             ⠈      │     ⠁
        └────────────────────────────────────────
         -1.836                             1.836
5 vertices on a circle, the first at the top and the others with the clock; 5 edges
```

### Pictures as files: PNG and JPEG XL

`save_png("name.png", drawing, choices)` and `save_jxl("name.jxl", drawing, choices)` save a
drawing as a picture of pixels; `save` takes the kind of file from the end of the name. The
drawing is any of the walks, spirals, plots and pictures above: `walk`, `walk3d`, `primewalk`,
`primewalk3d`, `primemodwalk`, `primemodwalk3d`, `ulamspiral`, `sacksspiral`, `plot`,
`parametricplot`, `polarplot`, `listplot`, `histogram`, `implicitplot`, `complexplot`,
`densityplot`, `plot3d`, `parametricplot3d`, `graphplot`, `matrixplot`, and the charts of data
(`bar`, `pie`, `scatter`, `surf`, ...; see [What MATLAB has](#what-matlab-has)), and the shapes
of four dimensions `plot4d` and `view4d` (below). The answer tells what was saved.

```
$ symbolic-math 'save_png("walk.png", walk(pi, 10, 10000))'
10000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees
from the origin to (-65.23, 76.9), 100.8 away; x from -83.73 to 34.73, y from -51.9 to 85.85
saved walk.png: PNG, 1024 x 1024 pixels, 8 bits, RGB, level 9, 256497 bytes
$ symbolic-math 'save_jxl("walk.jxl", walk(pi, 10, 10000))'
10000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees
from the origin to (-65.23, 76.9), 100.8 away; x from -83.73 to 34.73, y from -51.9 to 85.85
saved walk.jxl: JPEG XL, 1024 x 1024 pixels, 8 bits, RGB, lossless, effort 7, 207176 bytes
$ symbolic-math 'save_png("walk16.png", walk(pi), width = 800, height = 600, bits = 16, gray, level = 9)'
1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees
from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53
saved walk16.png: PNG, 800 x 600 pixels, 16 bits, gray, level 9, 85054 bytes
$ symbolic-math 'save_jxl("lossy.jxl", walk(pi), quality = 80, effort = 9)'
1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees
from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53
saved lossy.jxl: JPEG XL, 1024 x 1024 pixels, 8 bits, RGB, lossy at the distance 1.9, effort 9, 79618 bytes
$ symbolic-math 'save_png("turn.png", walk3d(e + pi*i, 12, 2000), frames = 12, fps = 6, width = 300)'
2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane
from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53
saved turn.png: PNG, 300 x 300 pixels, 8 bits, RGB, 12 frames, 6 a second, level 9, 401967 bytes
$ symbolic-math 'save_jxl("slices.jxl", walk3d(e + pi*i, 12, 2000), slices = 16, depth, width = 300)'
2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane
from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53
saved slices.jxl: JPEG XL, 300 x 300 pixels, 8 bits, RGB + depth, 16 slices as frames, 12 a second, lossless, effort 7, 71044 bytes
$ symbolic-math 'save_jxl("views.jxl", walk3d(e + pi*i, 12, 2000), views, width = 300)'
2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane
from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53
saved views.jxl: JPEG XL, 300 x 300 pixels, 8 bits, RGB, 3 views as frames (from above, from the front, from the right), 12 a second, lossless, effort 7, 74559 bytes
$ symbolic-math 'save_png("eyes.png", walk3d(e + pi*i, 12, 2000), stereo, width = 600)'
2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane
from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53
saved eyes.png: PNG, 600 x 600 pixels, 8 bits, red for the left eye and cyan for the right one, 4 degrees apart, level 9, 128146 bytes
$ symbolic-math 'save_png("plot.png", plot([sin(x), cos(x)], x, 0, 2pi), grid)'
saved plot.png: PNG, 1024 x 768 pixels, 8 bits, RGB, level 9, 36843 bytes
$ symbolic-math 'save_png("surface.png", plot3d(sin(x)*cos(y), x, -pi, pi, y, -pi, pi), azimuth = 45)'
the surface z = cos(y)*sin(x) over 128 by 128 cells; z from -1 to 1
saved surface.png: PNG, 1024 x 1024 pixels, 8 bits, RGB, level 9, 230069 bytes
```

What is chosen follows the drawing as `name = value` or as a word, in any order:

| Choice | Meaning |
| --- | --- |
| `width = 1024`, `height` | the size in pixels; the height follows from the drawing when it is left out (16384 a side and 16777216 pixels at most) |
| `bits = 8` | bits of a sample: 8 or 16 in a PNG file, 1 to 16 in a JPEG XL file |
| `gray`, `alpha`, `channels = n` | a gray picture; a channel that says how much of each pixel is drawn (no background); 1 gray, 2 gray + alpha, 3 colours, 4 colours + alpha |
| `level = 9` | PNG: how hard the file is packed, 0 (not at all) to 9 |
| `lossless`, `quality = q`, `distance = d` | JPEG XL: without loss (the usual), or with loss at a quality from 0 to 100 (90 is the distance 1) or at a distance from 0 to 25 |
| `effort = 7` | JPEG XL: the effort of the encoder, 1 to 10 |
| `depth` | JPEG XL: an extra channel with the depth of every pixel, 0 for the nearest thing and 1 for the farthest and for the background |
| `frames = n`, `fps = 12` | a moving picture (APNG or JPEG XL animation): the eye goes once around the drawing |
| `turn = degrees` | how far the eye goes around during the frames (360 when left out; 0 with `grow`) |
| `grow` | the walk grows from frame to frame |
| `slices = n` | the drawing cut into `n` slabs of depth, the nearest first, a frame each: the frames together are the drawing in space |
| `views` | three frames with one scale: the drawing from above, from the front and from the right |
| `stereo`, `stereo = degrees` | a picture for glasses with a red (left) and a cyan (right) glass: two eyes that stand 4 degrees (or as many as given) apart |
| `thickness = 1`, `margin = 0.04` | the width of a line in pixels; the free rim as a part of the smaller side |
| `palette = name` | the colours along a walk or of the values: `rainbow`, `fire`, `ice`, `viridis`, `plasma`, `magma`, `inferno`, `cividis`, `heat` (black, brown, red, orange, yellow, white, bluish), `thermal`, `ocean`, `sunset`, `twilight`, `coolwarm`, `earth`, `spectrum`, `cube` (all 512 colours of 3 bits a sample, each next to the one before), `gray`, `ink`, `cool`, `warm`, those of MATLAB (`parula`, `turbo`, `jet`, `hot`, `hsv`, `bone`, `copper`, `pink`, `spring`, `summer`, `autumn`, `winter`), or a list of colours, also written `{(0,0,0),(0,0,1),(1,1,1)}`; `reverse` turns it around |
| `color = c`, `background = c` | one colour for what is drawn; the ground. A colour is a name (`black`, `white`, `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `orange`, `purple`, `brown`, `pink`, `gray`, `navy`, `lime`, `gold`, `silver`), `"#rrggbb"`, or `[red, green, blue]` with numbers from 0 to 1 |
| `title = "text"`, `xlabel`, `ylabel` | a title above any picture; names of the axes of a chart of data |
| `axes`, `noaxes`, `grid` | axes with numbers (the usual for plots; a box with the ranges in space), none, faint lines at the numbers |
| `azimuth = 30`, `elevation = 25`, `roll = 0` | in space: the degrees by which the eye stands around the z-axis and above the plane, and by which the picture is turned (a drawing of the plane is seen from above: elevation 90) |
| `viewpoint = [x, y, z]` | the point of the eye: things that are near it are larger |
| `perspective = d` | the same from the direction of the angles, the eye `d` radii of the drawing away |
| `center = [x, y, z]`, `scale = s`, `zoom = z` | the point in the middle of the picture; pixels for a unit instead of a picture that is filled; a factor of the size |
| `fog = f` | in space: how much the farthest things fade into the background, 0 to 1 (0.5 for walks and curves, 0.2 for surfaces when it is left out) |

Pictures are made with the four operations and the square root of 64-bit floats, and with
sines and cosines of the arbitrary-precision library, so that a PNG file is the same on every
system; a JPEG XL file without loss holds the same pixels, and the size of one with loss may
differ by a few bytes between kinds of processors. A picture of a complex function, of a
density or of a surface works the function out once for every pixel or cell, with 64 bits.

### Functions of a complex number as shapes of four dimensions

`plot4d(f, z, a, b)` draws a function of a complex number in the box with the corners `a` and
`b` (`plot4d(f, z, r)`: the square of the radius `r`; `plot4d(f)`: from `-2 - 2i` to `2 + 2i`)
as a shape of four dimensions. A file of pixels has a width and a height, frames are the third
dimension and colours the fourth: the real and the imaginary part of `z` and of `f(z)` lie on
these four sides, and the alpha channel says which voxels belong to the shape. As a statement
`plot4d` shows the frame in the middle with characters; `save_jxl` and `save_png` save the whole
body as a moving JPEG XL or PNG picture.

```
$ symbolic-math 'plot4d(sin(z), width = 40, height = 12)'
                    @@
                    %%
                    ###
                    ****
                    ****+
                    +++++
                    +++++
                    ====+
                    ====
                    ---
                    ::
                    ..
plot4d of sin(z): frame 9 of 16, where the real part of f(z) is from 0 to 0.4701; the characters .:-=+*#%@ are the colours, the least value first
the width: the real part of z from -2 to 2; the height: the imaginary part of z from -2 to 2, upwards; the frames: the real part of f(z) from -3.761 to 3.761; the colours: the imaginary part of f(z) from -3.627 to 3.627
750 voxels of the box of 40 x 12 x 16 belong to the surface (9.766 %)
save_jxl("plot4d.jxl", plot4d(...)) saves the whole shape: 512 frames of 512 x 512 pixels in 512 colours, with alpha where the shape is
$ symbolic-math 'save_jxl("sin.jxl", plot4d(sin(z)), width = 64, height = 64, frames = 64)'
plot4d of sin(z)
the width: the real part of z from -2 to 2; the height: the imaginary part of z from -2 to 2, upwards; the frames: the real part of f(z) from -3.762 to 3.762; the colours: the imaginary part of f(z) from -3.627 to 3.627 (512 colours of the palette rainbow, the least value first)
6592 voxels of the box of 64 x 64 x 64 belong to the surface (2.515 %)
saved sin.jxl: JPEG XL, 64 x 64 pixels, 64 frames, 16 a second, 8 bits, RGB + alpha of 1 bit, lossless, effort 10, 21736 bytes
$ symbolic-math 'save_png("turned.png", plot4d(z^3 - 1, z, 1.5), fill = below, palette = cube, rotate(30, 20), width = 64, height = 64, frames = 32)'
plot4d of z^3 - 1
the width: the real part of z from -1.5 to 1.5; the height: the imaginary part of z from -1.5 to 1.5, upwards; the frames: the real part of f(z) from -7.75 to 5.75; the colours: the imaginary part of f(z) from -6.75 to 6.75 (512 colours of the palette cube, the least value first)
after that the body is turned and made smaller by the factor 0.6211, so that its box stays in the picture: about the middle of its box, first about the frames (roll), then about the width (pitch), then about the upright axis (yaw)
5436 voxels of the box of 64 x 64 x 32 belong to the surface (4.147 %); the body below it is filled
saved turned.png: PNG, 64 x 64 pixels, 32 frames, 16 a second, 8 bits, RGB + alpha, level 9, 35205 bytes
$ symbolic-math 'save_jxl("rings.jxl", plot4d(z*sin(z), z, 6), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, width = 64, height = 64, frames = 32)'
plot4d of z*sin(z)
the width: the real part of f(z) from -1652 to 1331; the height: the imaginary part of f(z) from -1471 to 1471, upwards; the frames: the imaginary part of z from -6 to 6; the colours: the real part of z from -6 to 6 (512 colours of the palette rainbow, the least value first)
every frame is stretched on its own (normalize): what it holds fills the picture and takes the whole palette, so the frames together are no body of one scale any more
30 of 32 frames became larger; the others reach from edge to edge both ways as they are
61012 voxels of the box of 64 x 64 x 32 belong to the surface (46.55 %)
saved rings.jxl: JPEG XL, 64 x 64 pixels, 32 frames, 16 a second, 8 bits, RGB + alpha of 1 bit, lossless, effort 10, 56098 bytes
$ symbolic-math 'save_jxl_fast("quick.jxl", plot4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat, width = 64, height = 64, frames = 32)'
plot4d of gamma(z)
the width: the real part of z from -4 to 4; the height: the imaginary part of z from -3 to 3, upwards; the frames: the real part of f(z) from -8.717 to 7.749; the colours: the imaginary part of f(z) from -3.298 to 3.298 (256 colours of the palette heat, the least value first)
the imaginary part of f(z) goes far beyond these ends (near a pole): what lies outside is left out; range_imag = [least, greatest] sets other ends
4818 voxels of the box of 64 x 64 x 32 belong to the surface (3.676 %); the body below it is filled
saved quick.jxl: JPEG XL, 64 x 64 pixels, 32 frames, 16 a second, 8 bits, RGB + alpha of 1 bit, lossy at the distance 1 (alpha stays exact), effort 3, 32971 bytes
$ symbolic-math 'imframes("sin.jxl"); nnz(last(imread("sin.jxl", 32)))'
64
96
```

| Choice | Meaning |
| --- | --- |
| `arg_real`, `arg_imag`, `val_real`, `val_imag` `= width`, `height`, `frame` or `color` | which part of `z` and of `f(z)` lies on which side; the usual ones are `arg_real = width, arg_imag = height, val_real = frame, val_imag = color`, and parts that are not said take the sides that are left |
| `width`, `height`, `frames`, `colors` | the sizes of the four sides: 512 each unless said |
| `palette = name` or a list of colours | the colours from the least value to the largest (`rainbow` unless said) |
| `bits`, `gray`, `channels` | RGB with 8 bits and one bit of alpha unless said; `bits = 3` holds 512 colours, `gray, bits = 9` 512 levels in one channel |
| `fill = surface`, `below`, `above` | the surface alone, or the body between it and the least (largest) value of the side of the frames |
| `thickness = n` | a surface of `n` voxels |
| `range_real = [a, b]`, `range_imag = [a, b]` | the values of `f(z)` that are drawn; without them the values near a pole are left out by themselves |
| `rotate(yaw, pitch, roll)` | the body turned by these degrees before it is saved (made smaller so that it stays in its box) |
| `normalize` | every frame is stretched on its own to the whole picture and the whole palette: more to see in every frame, and a body like a tunnel instead of a box |
| `fps = 16`, `lossless`, `quality`, `distance`, `effort`, `level` | as for every picture: without loss and with the hardest packing unless said |

The body of `512 x 512 x 512` voxels that is saved without sizes takes a minute or more with
the hardest packing. `save_png_fast`, `save_jxl_fast` and `save_fast` are the same commands
for a quick look: 256 pixels wide (a shape of four dimensions: 256 on every side), a PNG file
packed at `level = 1`, a JPEG XL file with loss at the distance 1 and `effort = 3`; every
choice may still be given. `imread("file", k)` reads frame `k` of a moving picture and
`imframes("file")` counts its frames.

### The shape seen from outside

`view4d(f, z, a, b)` takes the same body of voxels and draws one picture of it as an eye in
space sees it from outside: what is hidden is left out, and the surface is lighter where it
faces the light. The frames of `plot4d` stand upright, so that with the usual sides the real
part of `f(z)` stands as heights over the plane of `z` in the colours of the imaginary part -
mountains over a carpet. The program finds a place for the eye that shows much of the shape;
`azimuth`, `elevation`, `viewpoint`, `perspective`, `roll` and `zoom` put it elsewhere, as for
every drawing of space, and with `frames = n` the eye goes around the body. Every choice of the
shape of `plot4d` is taken (`fill`, `thickness`, the sides, `range_real`, `range_imag`, `rotate`,
`normalize`, `colors`, `palette`), and `voxels = n` or `voxels = [across, upwards, frames]` says
of how many cubes the body is made: half as many along a side as the picture has pixels across
unless said, 512 at most. As a statement `view4d` is a picture of characters.

```
$ symbolic-math 'view4d(z^2, width = 40, height = 12)'

                   ⠄⠶⣶⣶⣶⣶⠴⠄
                 ⠄⠴⣶⣶⣶⣶⣶⣶⣶⣶⠶
             ⠄⠄⠄⠄⠶⣶⣶⣶⣾⣾⣾⣶⣶⣶⣶⠶
           ⠄⠤⠴⠴⠴⠴⠶⡶⣶⣾⣾⣾⣶⣶⣶⣶⣶⡶⡶⠄
          ⠄⠴⠴⠴⠴⠴⠴⠴⠴⠶⡶⣾⣶⣶⣶⣶⣶⡶⡶⡶⠴
         ⠄⠴⠴⠴⠤⠤⠤⠴⠄⠄⠄⠴⡶⡶⣶⡶⡶⡶⡶⡶⡶⠄
         ⠄⠤⠤⠤⠤⠤⠄     ⠴⠶⠶⡶⡶⡶⡶⡶⠴
          ⠄⠤⠤⠤⠄       ⠤⠴⠶⠶⠶⡶⠶
                       ⠤⠴⠴⠶⠶⠄
                        ⠄⠄⠄⠄

view4d of z^2: the shape of plot4d as an eye outside of its box sees it; the frames of plot4d stand upright
the box: x is the real part of z from -2 to 2; y is the imaginary part of z from -2 to 2; z, upright, is the real part of f(z) from -4 to 4; the colours are the imaginary part of f(z) from -8 to 8
3774 voxels of the box of 40 x 40 x 40 belong to the surface (5.897 %)
the body has 3774 cubes, of which 9796 sides lie open
the eye stands at azimuth = 120, elevation = 25 (found: the lowest of three heights that shows most of what can be seen, and around the box the place that shows most); azimuth, elevation, roll, viewpoint, perspective and zoom choose another
save_png("view4d.png", view4d(...)) saves the picture in colours: 1024 x 1024 pixels of a body of 512 x 512 x 512 voxels
$ symbolic-math 'save_png("sin.png", view4d(sin(z)), width = 160)'
view4d of sin(z): the shape of plot4d as an eye outside of its box sees it; the frames of plot4d stand upright
the box: x is the real part of z from -2 to 2; y is the imaginary part of z from -2 to 2; z, upright, is the real part of f(z) from -3.761 to 3.761; the colours are the imaginary part of f(z) from -3.627 to 3.627 (512 colours of the palette rainbow, the least value first)
10457 voxels of the box of 80 x 80 x 80 belong to the surface (2.042 %)
the body has 10457 cubes, of which 26586 sides lie open
the eye stands at azimuth = 240, elevation = 25 (found: the lowest of three heights that shows most of what can be seen, and around the box the place that shows most); azimuth, elevation, roll, viewpoint, perspective and zoom choose another
saved sin.png: PNG, 160 x 160 pixels, 8 bits, RGB, level 9, 7438 bytes
$ symbolic-math 'save_png("gamma.png", view4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat, azimuth = 60, elevation = 35, perspective = 3, width = 160)'
view4d of gamma(z): the shape of plot4d as an eye outside of its box sees it; the frames of plot4d stand upright
the box: x is the real part of z from -4 to 4; y is the imaginary part of z from -3 to 3; z, upright, is the real part of f(z) from -2.9 to 3.863; the colours are the imaginary part of f(z) from -3.196 to 3.196 (512 colours of the palette heat, the least value first)
the real part of f(z) goes far beyond these ends (near a pole): what lies outside is left out; range_real = [least, greatest] sets other ends
the imaginary part of f(z) goes far beyond these ends (near a pole): what lies outside is left out; range_imag = [least, greatest] sets other ends
12679 voxels of the box of 80 x 80 x 80 belong to the surface (2.476 %); the body below it is filled
the body has 230503 cubes, of which 34796 sides lie open
saved gamma.png: PNG, 160 x 160 pixels, 8 bits, RGB, level 9, 7219 bytes
$ symbolic-math 'save_jxl("around.jxl", view4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma, frames = 8, width = 96)'
view4d of (z^3 - 1)/(z^3 + 1): the shape of plot4d as an eye outside of its box sees it; the frames of plot4d stand upright
the box: x is the real part of z from -2 to 2; y is the imaginary part of z from -2 to 2; z, upright, is the real part of f(z) from -2.295 to 2.853; the colours are the imaginary part of f(z) from -2.362 to 2.362 (512 colours of the palette magma, the least value first)
the real part of f(z) goes far beyond these ends (near a pole): what lies outside is left out; range_real = [least, greatest] sets other ends
the imaginary part of f(z) goes far beyond these ends (near a pole): what lies outside is left out; range_imag = [least, greatest] sets other ends
4651 voxels of the box of 48 x 48 x 48 belong to the surface (4.206 %); the body below it is filled
the body has 63508 cubes, of which 15566 sides lie open
the eye stands at azimuth = 30, elevation = 25 (found: the lowest of three heights that shows most of what can be seen, and around the box the place that shows most); azimuth, elevation, roll, viewpoint, perspective and zoom choose another
saved around.jxl: JPEG XL, 96 x 96 pixels, 8 bits, RGB, 8 frames, 12 a second, lossless, effort 7, 5757 bytes
$ symbolic-math 'save_png_fast("rings.png", view4d(z*sin(z), z, 6), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2)'
view4d of z*sin(z): the shape of plot4d as an eye outside of its box sees it; the frames of plot4d stand upright
the box: x is the real part of f(z) from -1652 to 1331; y is the imaginary part of f(z) from -1471 to 1471; z, upright, is the imaginary part of z from -6 to 6; the colours are the real part of z from -6 to 6 (256 colours of the palette rainbow, the least value first)
every frame is stretched on its own (normalize): what it holds fills the picture and takes the whole palette, so the frames together are no body of one scale any more
126 of 128 frames became larger; the others reach from edge to edge both ways as they are
473076 voxels of the box of 128 x 128 x 128 belong to the surface (22.56 %)
the body has 517864 cubes, of which 258542 sides lie open
the eye stands at azimuth = 30, elevation = 25 (found: the lowest of three heights that shows most of what can be seen, and around the box the place that shows most); azimuth, elevation, roll, viewpoint, perspective and zoom choose another
saved rings.png: PNG, 256 x 256 pixels, 8 bits, RGB, level 1, 29509 bytes
```

### Pictures made with it

The directory [jxls](jxls) holds pictures that the program made, all without loss and packed
as hard as the encoder can (`effort = 10`); `sh jxls/make.sh` makes them again (a file that is
there is left as it is). They are JPEG XL files: a viewer that knows the format shows them,
the moving ones with their frames.

<!-- pictures -->
**Shapes of four dimensions (`plot4d`: 512 x 512 pixels, 512 frames, 512 colours; alpha says where the shape is)**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [plot4d_sin.jxl](jxls/plot4d_sin.jxl) | `save_jxl("plot4d_sin.jxl", plot4d(sin(z)))` |
| [plot4d_gamma_body.jxl](jxls/plot4d_gamma_body.jxl) | `save_jxl("plot4d_gamma_body.jxl", plot4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat)` |
| [plot4d_three_poles_body.jxl](jxls/plot4d_three_poles_body.jxl) | `save_jxl("plot4d_three_poles_body.jxl", plot4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma)` |
| [plot4d_tan.jxl](jxls/plot4d_tan.jxl) | `save_jxl("plot4d_tan.jxl", plot4d(tan(z), z, 3), range_real = [-3, 3], range_imag = [-3, 3], palette = turbo)` |
| [plot4d_zeta_body.jxl](jxls/plot4d_zeta_body.jxl) | `save_jxl("plot4d_zeta_body.jxl", plot4d(zeta(z), z, -6 - 12i, 6 + 12i), fill = below, palette = inferno, range_real = [-3, 3], range_imag = [-3, 3])` |
| [plot4d_waves_body.jxl](jxls/plot4d_waves_body.jxl) | `save_jxl("plot4d_waves_body.jxl", plot4d(sin(z)*cos(z), z, pi), fill = below, palette = twilight, range_real = [-2, 2], range_imag = [-2, 2])` |
| [plot4d_ribbons.jxl](jxls/plot4d_ribbons.jxl) | `save_jxl("plot4d_ribbons.jxl", plot4d(z^5 - z, z, 1.5), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2)` |
| [plot4d_rings.jxl](jxls/plot4d_rings.jxl) | `save_jxl("plot4d_rings.jxl", plot4d(z*sin(z), z, 6), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2)` |
| [plot4d_circles.jxl](jxls/plot4d_circles.jxl) | `save_jxl("plot4d_circles.jxl", plot4d(e^z, z, -2 - pi*i, 2 + pi*i), arg_real = frame, arg_imag = color, val_real = width, val_imag = height, normalize, thickness = 3)` |
| [plot4d_bows.jxl](jxls/plot4d_bows.jxl) | `save_jxl("plot4d_bows.jxl", plot4d(z^2, z, 2), arg_real = frame, arg_imag = color, val_real = width, val_imag = height, normalize, thickness = 2, palette = spectrum)` |
| [plot4d_moebius.jxl](jxls/plot4d_moebius.jxl) | `save_jxl("plot4d_moebius.jxl", plot4d((z - 1)/(z + 1), z, 3), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, range_real = [-3, 5], range_imag = [-4, 4], thickness = 2)` |
| [plot4d_cube_bands.jxl](jxls/plot4d_cube_bands.jxl) | `save_jxl("plot4d_cube_bands.jxl", plot4d(z^3 - 1, z, 1.5), fill = below, palette = cube, normalize)` |
| [plot4d_turned_sheet.jxl](jxls/plot4d_turned_sheet.jxl) | `save_jxl("plot4d_turned_sheet.jxl", plot4d(sin(z), z, pi), rotate(35, 25, 10), thickness = 2, palette = turbo)` |
| [plot4d_curves.jxl](jxls/plot4d_curves.jxl) | `save_jxl("plot4d_curves.jxl", plot4d(sin(z)*cos(z)), arg_imag = frame, val_real = height, normalize, thickness = 3)` |
| [plot4d_gray9.jxl](jxls/plot4d_gray9.jxl) | `save_jxl("plot4d_gray9.jxl", plot4d(z + 1/z), gray, bits = 9, range_real = [-4, 4], range_imag = [-4, 4])` |
| [plot4d_cube3.jxl](jxls/plot4d_cube3.jxl) | `save_jxl("plot4d_cube3.jxl", plot4d(exp(z), z, 3), palette = cube, bits = 3, fill = below)` |

**The shapes seen from outside (`view4d`: one picture of a body of 512 x 512 x 512 voxels)**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [view4d_sin.jxl](jxls/view4d_sin.jxl) | `save_jxl("view4d_sin.jxl", view4d(sin(z)), width = 1920, height = 1080, effort = 10)` |
| [view4d_gamma.jxl](jxls/view4d_gamma.jxl) | `save_jxl("view4d_gamma.jxl", view4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat, width = 1920, height = 1080, effort = 10)` |
| [view4d_three_poles.jxl](jxls/view4d_three_poles.jxl) | `save_jxl("view4d_three_poles.jxl", view4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma, range_real = [-3, 3], perspective = 3, width = 1920, height = 1080, effort = 10)` |
| [view4d_tan.jxl](jxls/view4d_tan.jxl) | `save_jxl("view4d_tan.jxl", view4d(tan(z), z, 3), range_real = [-3, 3], range_imag = [-3, 3], palette = turbo, width = 1920, height = 1080, effort = 10)` |
| [view4d_zeta.jxl](jxls/view4d_zeta.jxl) | `save_jxl("view4d_zeta.jxl", view4d(zeta(z), z, -6 - 12i, 6 + 12i), fill = below, palette = inferno, range_real = [-3, 3], range_imag = [-3, 3], voxels = [256, 512, 256], width = 1920, height = 1080, effort = 10)` |
| [view4d_carpet.jxl](jxls/view4d_carpet.jxl) | `save_jxl("view4d_carpet.jxl", view4d(z + 1/z), range_real = [-4, 4], range_imag = [-4, 4], palette = turbo, azimuth = 30, elevation = 60, width = 1920, height = 1080, effort = 10)` |
| [view4d_waves.jxl](jxls/view4d_waves.jxl) | `save_jxl("view4d_waves.jxl", view4d(sin(z)*cos(z), z, pi), palette = twilight, range_real = [-2, 2], range_imag = [-2, 2], width = 1920, height = 1080, effort = 10)` |
| [view4d_ribbons.jxl](jxls/view4d_ribbons.jxl) | `save_jxl("view4d_ribbons.jxl", view4d(z^5 - z, z, 1.5), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, thickness = 2, width = 1920, height = 1080, effort = 10)` |
| [view4d_rings.jxl](jxls/view4d_rings.jxl) | `save_jxl("view4d_rings.jxl", view4d(z*sin(z), z, 6), arg_real = color, arg_imag = frame, val_real = width, val_imag = height, normalize, thickness = 2, width = 1920, height = 1080, effort = 10)` |
| [view4d_root.jxl](jxls/view4d_root.jxl) | `save_jxl("view4d_root.jxl", view4d(sqrt(z)), val_real = color, val_imag = frame, background = white, width = 1920, height = 1080, effort = 10)` |
| [view4d_hang.jxl](jxls/view4d_hang.jxl) | `save_jxl("view4d_hang.jxl", view4d(z^4 - 1, z, 1.3), fill = above, palette = plasma, width = 1920, height = 1080, effort = 10)` |
| [view4d_two_poles_stereo.jxl](jxls/view4d_two_poles_stereo.jxl) | `save_jxl("view4d_two_poles_stereo.jxl", view4d(1/(z^2 + 1), z, 2), range_real = [-2, 2], range_imag = [-2, 2], stereo = 5, width = 1920, height = 1080, effort = 10)` |
| [view4d_erf_turned.jxl](jxls/view4d_erf_turned.jxl) | `save_jxl("view4d_erf_turned.jxl", view4d(erf(z), z, 2), fill = below, rotate(25, 20, 0), palette = coolwarm, range_real = [-3, 3], range_imag = [-3, 3], width = 1920, height = 1080, effort = 10)` |
| [view4d_exp.jxl](jxls/view4d_exp.jxl) | `save_jxl("view4d_exp.jxl", view4d(e^z, z, -2 - pi*i, 2 + pi*i), title = "the exponential function", width = 1920, height = 1080, effort = 10)` |

**Walks of digits and of primes, spirals**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [walk_pi.jxl](jxls/walk_pi.jxl) | `save_jxl("walk_pi.jxl", walk(pi, 10, 200000), width = 1920, height = 1080, effort = 10)` |
| [walk_e_base4.jxl](jxls/walk_e_base4.jxl) | `save_jxl("walk_e_base4.jxl", walk(e, 4, 300000), palette = fire, width = 1920, height = 1080, effort = 10)` |
| [walk_sqrt2_base6.jxl](jxls/walk_sqrt2_base6.jxl) | `save_jxl("walk_sqrt2_base6.jxl", walk(sqrt(2), 6, 200000), palette = viridis, width = 1920, height = 1080, effort = 10)` |
| [primewalk.jxl](jxls/primewalk.jxl) | `save_jxl("primewalk.jxl", primewalk(200000), width = 1920, height = 1080, effort = 10)` |
| [primemodwalk.jxl](jxls/primemodwalk.jxl) | `save_jxl("primemodwalk.jxl", primemodwalk(300000, 12), width = 1920, height = 1080, effort = 10)` |
| [ulamspiral.jxl](jxls/ulamspiral.jxl) | `save_jxl("ulamspiral.jxl", ulamspiral(500000), width = 1920, height = 1080, effort = 10)` |
| [sacksspiral.jxl](jxls/sacksspiral.jxl) | `save_jxl("sacksspiral.jxl", sacksspiral(200000), width = 1920, height = 1080, effort = 10)` |
| [walk3d_near.jxl](jxls/walk3d_near.jxl) | `save_jxl("walk3d_near.jxl", walk3d(e + pi*i, 12, 100000), perspective = 3, fog = 0.8, width = 1920, height = 1080, effort = 10)` |
| [walk3d_stereo.jxl](jxls/walk3d_stereo.jxl) | `save_jxl("walk3d_stereo.jxl", walk3d(sqrt(2) + sqrt(3)*i, 12, 60000), stereo, width = 1920, height = 1080, effort = 10)` |
| [walk3d_depth.jxl](jxls/walk3d_depth.jxl) | `save_jxl("walk3d_depth.jxl", walk3d(e + pi*i, 12, 100000), depth, azimuth = 300, elevation = 35, width = 1920, height = 1080, effort = 10)` |
| [primewalk3d.jxl](jxls/primewalk3d.jxl) | `save_jxl("primewalk3d.jxl", primewalk3d(100000), axes, width = 1920, height = 1080, effort = 10)` |
| [primemodwalk3d.jxl](jxls/primemodwalk3d.jxl) | `save_jxl("primemodwalk3d.jxl", primemodwalk3d(100000), width = 1920, height = 1080, effort = 10)` |
| [primes3d_xyz.jxl](jxls/primes3d_xyz.jxl) | `save_jxl("primes3d_xyz.jxl", primes3d_xyz(20000, 4), width = 1920, height = 1080, effort = 10)` |

**Surfaces and curves of space**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [plot3d_ripples.jxl](jxls/plot3d_ripples.jxl) | `save_jxl("plot3d_ripples.jxl", plot3d(sin(sqrt(x^2 + y^2))/sqrt(x^2 + y^2), x, -12, 12, y, -12, 12), palette = turbo, perspective = 3, width = 1920, height = 1080, effort = 10)` |
| [plot3d_saddle.jxl](jxls/plot3d_saddle.jxl) | `save_jxl("plot3d_saddle.jxl", plot3d(x^2 - y^2, x, -2, 2, y, -2, 2), palette = turbo, width = 1920, height = 1080, effort = 10)` |
| [surf_peaks.jxl](jxls/surf_peaks.jxl) | `save_jxl("surf_peaks.jxl", surf(peaks(80)), palette = turbo, azimuth = 50, elevation = 30, width = 1920, height = 1080, effort = 10)` |
| [surfc_peaks.jxl](jxls/surfc_peaks.jxl) | `save_jxl("surfc_peaks.jxl", surfc(peaks(60)), palette = jet, width = 1920, height = 1080, effort = 10)` |
| [waterfall_peaks.jxl](jxls/waterfall_peaks.jxl) | `save_jxl("waterfall_peaks.jxl", waterfall(peaks(40)), width = 1920, height = 1080, effort = 10)` |
| [hyperboloid.jxl](jxls/hyperboloid.jxl) | `save_jxl("hyperboloid.jxl", fimplicit3(x^2 + y^2 - z^2 = 1, x, -2, 2, y, -2, 2, z, -2, 2), width = 1920, height = 1080, effort = 10)` |
| [contour3_cells.jxl](jxls/contour3_cells.jxl) | `save_jxl("contour3_cells.jxl", contour3(sin(x)*cos(y), x, -pi, pi, y, -pi, pi), width = 1920, height = 1080, effort = 10)` |
| [bar3_magic.jxl](jxls/bar3_magic.jxl) | `save_jxl("bar3_magic.jxl", bar3(magic(6)), width = 1920, height = 1080, effort = 10)` |
| [helix.jxl](jxls/helix.jxl) | `save_jxl("helix.jxl", scatter3(cos((0:400)/10), sin((0:400)/10), (0:400)/40), width = 1920, height = 1080, effort = 10)` |
| [knot.jxl](jxls/knot.jxl) | `save_jxl("knot.jxl", parametricplot3d([(2 + cos(3t))*cos(2t), (2 + cos(3t))*sin(2t), sin(3t)], t, 0, 2pi), thickness = 3, perspective = 4, width = 1920, height = 1080, effort = 10)` |

**Functions of a complex number in colours, charts**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [complexplot_cubic.jxl](jxls/complexplot_cubic.jxl) | `save_jxl("complexplot_cubic.jxl", complexplot(z^3 - 1, z, -2.4 - 1.35i, 2.4 + 1.35i), width = 1920, height = 1080, effort = 10)` |
| [complexplot_gamma.jxl](jxls/complexplot_gamma.jxl) | `save_jxl("complexplot_gamma.jxl", complexplot(gamma(z), z, -6.4 - 3.6i, 6.4 + 3.6i), width = 1920, height = 1080, effort = 10)` |
| [complexplot_zeta.jxl](jxls/complexplot_zeta.jxl) | `save_jxl("complexplot_zeta.jxl", complexplot(zeta(z), z, -16 - 9i, 16 + 9i), width = 1920, height = 1080, effort = 10)` |
| [complexplot_essential.jxl](jxls/complexplot_essential.jxl) | `save_jxl("complexplot_essential.jxl", complexplot(sin(1/z), z, -0.8 - 0.45i, 0.8 + 0.45i), width = 1920, height = 1080, effort = 10)` |
| [complexplot_blaschke.jxl](jxls/complexplot_blaschke.jxl) | `save_jxl("complexplot_blaschke.jxl", complexplot((z^2 - 1)/(z^2 + 1), z, -3.2 - 1.8i, 3.2 + 1.8i), width = 1920, height = 1080, effort = 10)` |
| [densityplot_waves.jxl](jxls/densityplot_waves.jxl) | `save_jxl("densityplot_waves.jxl", densityplot(sin(x)*cos(y) + sin(x*y/3), x, -8, 8, y, -4.5, 4.5), palette = twilight, width = 1920, height = 1080, effort = 10)` |
| [contour_cells.jxl](jxls/contour_cells.jxl) | `save_jxl("contour_cells.jxl", contour(sin(x)*cos(y), x, -2pi, 2pi, y, -pi, pi), width = 1920, height = 1080, effort = 10)` |
| [implicitplot_heart.jxl](jxls/implicitplot_heart.jxl) | `save_jxl("implicitplot_heart.jxl", implicitplot((x^2 + y^2 - 1)^3 = x^2*y^3, x, -2.4, 2.4, y, -1.2, 1.5), width = 1920, height = 1080, effort = 10)` |
| [streamline_spiral.jxl](jxls/streamline_spiral.jxl) | `save_jxl("streamline_spiral.jxl", streamline([-y, x + y/2], x, -2, 2, y, -1.2, 1.2), width = 1920, height = 1080, effort = 10)` |
| [plot_four_curves.jxl](jxls/plot_four_curves.jxl) | `save_jxl("plot_four_curves.jxl", plot([sin(x), cos(x), sin(2x)/2, x*cos(3x)/6], x, -2pi, 2pi), grid, title = "four curves", width = 1920, height = 1080, effort = 10)` |
| [parametricplot_lissajous.jxl](jxls/parametricplot_lissajous.jxl) | `save_jxl("parametricplot_lissajous.jxl", parametricplot([sin(3t), sin(4t)], t, 0, 2pi), width = 1920, height = 1080, effort = 10)` |
| [polarplot_rose.jxl](jxls/polarplot_rose.jxl) | `save_jxl("polarplot_rose.jxl", polarplot(cos(7t/4), t, 0, 8pi), width = 1920, height = 1080, effort = 10)` |
| [bodeplot.jxl](jxls/bodeplot.jxl) | `save_jxl("bodeplot.jxl", bodeplot(tf([1], [1, 0.4, 1])), width = 1920, height = 1080, effort = 10)` |
| [nyquistplot.jxl](jxls/nyquistplot.jxl) | `save_jxl("nyquistplot.jxl", nyquistplot(tf([4], [1, 3, 3, 1])), width = 1920, height = 1080, effort = 10)` |
| [rlocusplot.jxl](jxls/rlocusplot.jxl) | `save_jxl("rlocusplot.jxl", rlocusplot(tf([1, 2], [1, 3, 5, 0])), width = 1920, height = 1080, effort = 10)` |
| [nicholsplot.jxl](jxls/nicholsplot.jxl) | `save_jxl("nicholsplot.jxl", nicholsplot(tf([4], [1, 3, 3, 1])), width = 1920, height = 1080, effort = 10)` |
| [graphplot_petersen.jxl](jxls/graphplot_petersen.jxl) | `save_jxl("graphplot_petersen.jxl", graphplot(petersengraph()), width = 1920, height = 1080, effort = 10)` |

**Moving pictures: 512 frames of 512 x 512 pixels (the eye goes once around, a walk grows, space is cut into slabs)**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [view4d_around_sin.jxl](jxls/view4d_around_sin.jxl) | `save_jxl("view4d_around_sin.jxl", view4d(sin(z)), width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [view4d_around_gamma.jxl](jxls/view4d_around_gamma.jxl) | `save_jxl("view4d_around_gamma.jxl", view4d(gamma(z), z, -4 - 3i, 4 + 3i), fill = below, palette = heat, width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [view4d_around_poles.jxl](jxls/view4d_around_poles.jxl) | `save_jxl("view4d_around_poles.jxl", view4d((z^3 - 1)/(z^3 + 1), z, 2), fill = below, palette = magma, range_real = [-3, 3], width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [walk_pi_grows.jxl](jxls/walk_pi_grows.jxl) | `save_jxl("walk_pi_grows.jxl", walk(pi, 10, 100000), grow, width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [walk3d_around.jxl](jxls/walk3d_around.jxl) | `save_jxl("walk3d_around.jxl", walk3d(e + pi*i, 12, 50000), width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [walk3d_slices.jxl](jxls/walk3d_slices.jxl) | `save_jxl("walk3d_slices.jxl", walk3d(e + pi*i, 12, 100000), slices = 512, width = 512, height = 512, fps = 24, effort = 10)` |
| [plot3d_around.jxl](jxls/plot3d_around.jxl) | `save_jxl("plot3d_around.jxl", plot3d(sin(sqrt(x^2 + y^2))/sqrt(x^2 + y^2), x, -12, 12, y, -12, 12), palette = turbo, width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [knot_around.jxl](jxls/knot_around.jxl) | `save_jxl("knot_around.jxl", parametricplot3d([(2 + cos(3t))*cos(2t), (2 + cos(3t))*sin(2t), sin(3t)], t, 0, 2pi), thickness = 3, width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |

**Tetration `b^^h`, its inverse in the height `slog` and in the base `ssrt` (these ask the program `tet` for their values)**

| File | Made with `symbolic-math '...'` |
| --- | --- |
| [tetration_curves.jxl](jxls/tetration_curves.jxl) | `save_jxl("tetration_curves.jxl", plot([2^^x, e^^x, sqrt(2)^^x, 1.2^^x], x, -1.9, 2.2, [-4, 12]), grid, title = "b^^x for b = 2, e, sqrt(2) and 1.2", width = 1920, height = 1080, effort = 10)` |
| [slog_curves.jxl](jxls/slog_curves.jxl) | `save_jxl("slog_curves.jxl", plot([slog(2, x), slog(e, x), slog(10, x)], x, 0.05, 30), grid, title = "slog(b, x) for b = 2, e and 10", width = 1920, height = 1080, effort = 10)` |
| [ssrt_curves.jxl](jxls/ssrt_curves.jxl) | `save_jxl("ssrt_curves.jxl", plot([ssrt(x, 2), ssrt(x, 3), ssrt(x, 4)], x, 1, 30), grid, title = "ssrt(x, n) for n = 2, 3 and 4", width = 1920, height = 1080, effort = 10)` |
| [tetration_surface.jxl](jxls/tetration_surface.jxl) | `save_jxl("tetration_surface.jxl", plot3d(b^^h, b, 1.05, 1.44, h, -1.5, 8), palette = turbo, width = 1920, height = 1080, effort = 10)` |
| [tetration_surface_kneser.jxl](jxls/tetration_surface_kneser.jxl) | `save_jxl("tetration_surface_kneser.jxl", plot3d(b^^h, b, 1.6, 3, h, -1.5, 1.5), palette = plasma, width = 1920, height = 1080, effort = 10)` |
| [slog_surface.jxl](jxls/slog_surface.jxl) | `save_jxl("slog_surface.jxl", plot3d(slog(b, x), b, 1.1, 1.44, x, -0.5, 1.05), palette = viridis, width = 1920, height = 1080, effort = 10)` |
| [plot4d_tetration.jxl](jxls/plot4d_tetration.jxl) | `save_jxl("plot4d_tetration.jxl", plot4d(sqrt(2)^^z, z, -3.5 - 4i, 4.5 + 4i), range_real = [-3, 5], range_imag = [-4, 4])` |
| [plot4d_tetration_body.jxl](jxls/plot4d_tetration_body.jxl) | `save_jxl("plot4d_tetration_body.jxl", plot4d(sqrt(2)^^z, z, -3.5 - 4i, 4.5 + 4i), range_real = [-3, 5], range_imag = [-4, 4], fill = below, palette = magma)` |
| [view4d_tetration.jxl](jxls/view4d_tetration.jxl) | `save_jxl("view4d_tetration.jxl", view4d(sqrt(2)^^z, z, -3.5 - 4i, 4.5 + 4i), range_real = [-3, 5], range_imag = [-4, 4], palette = turbo, width = 1920, height = 1080, effort = 10)` |
| [view4d_tetration_body.jxl](jxls/view4d_tetration_body.jxl) | `save_jxl("view4d_tetration_body.jxl", view4d(sqrt(2)^^z, z, -3.5 - 4i, 4.5 + 4i), range_real = [-3, 5], range_imag = [-4, 4], fill = below, palette = magma, width = 1920, height = 1080, effort = 10)` |
| [view4d_tetration_e.jxl](jxls/view4d_tetration_e.jxl) | `save_jxl("view4d_tetration_e.jxl", view4d(e^^z, z, -1.8 - 2i, 2.2 + 2i), voxels = 192, range_real = [-4, 8], range_imag = [-6, 6], palette = plasma, width = 1920, height = 1080, effort = 10)` |
| [view4d_slog.jxl](jxls/view4d_slog.jxl) | `save_jxl("view4d_slog.jxl", view4d(slog(sqrt(2), z), z, -2 - 2i, 1.9 + 2i), voxels = 256, range_real = [-3, 6], palette = twilight, width = 1920, height = 1080, effort = 10)` |
| [view4d_ssrt.jxl](jxls/view4d_ssrt.jxl) | `save_jxl("view4d_ssrt.jxl", view4d(ssrt(z, 2), z, -4 - 4i, 6 + 4i), voxels = 256, palette = spectrum, width = 1920, height = 1080, effort = 10)` |
| [view4d_around_tetration.jxl](jxls/view4d_around_tetration.jxl) | `save_jxl("view4d_around_tetration.jxl", view4d(sqrt(2)^^z, z, -3.5 - 4i, 4.5 + 4i), range_real = [-3, 5], range_imag = [-4, 4], fill = below, palette = magma, width = 512, height = 512, frames = 512, fps = 24, effort = 10)` |
| [complexplot_tetration.jxl](jxls/complexplot_tetration.jxl) | `save_jxl("complexplot_tetration.jxl", complexplot(sqrt(2)^^z, z, -4.8 - 2.7i, 4.8 + 2.7i), width = 1920, height = 1080, effort = 10)` |
| [complexplot_tetration_fifth.jxl](jxls/complexplot_tetration_fifth.jxl) | `save_jxl("complexplot_tetration_fifth.jxl", complexplot(0.2^^z, z, -4 - 2.25i, 4 + 2.25i), width = 1920, height = 1080, effort = 10)` |
<!-- /pictures -->

## What MATLAB has

Most of what MATLAB computes with numbers, matrices, data, texts and functions is here under
MATLAB's own names (aliases of the names that this program had before, where it had the thing
already). `--functions` lists every name and `--help` has an example of each. The way of
writing stays the one of this program; MATLAB's forms `[1, 2; 3, 4]`, `1:5`, `.*`, `A \ b`,
`@(x) x^2`, `~=` are read too. Numbers are exact where MATLAB rounds to 16 digits: `inv`, `lu`,
`polyfit`, `linprog` or `ranksum` answer with fractions and roots when the input has them, and
with 32 digits or more when it has decimals.

### Arrays

A list is a row, a list of rows is a matrix, a list of matrices an array of three sizes; places
count from 1. The operators work on whole lists, `.*`, `./`, `.^` entry by entry.

```
'1:5'                                        [1, 2, 3, 4, 5]
'0:0.25:1'                                   [0, 0.25, 0.5, 0.75, 1]
'[1, 2; 3, 4] .* [10, 20; 30, 40]'           [[10, 40], [90, 160]]
'[1, 2; 3, 4] * [10, 20; 30, 40]'            [[70, 100], [150, 220]]
'reshape(1:6, 2, 3)'                         [[1, 2, 3], [4, 5, 6]]
'repmat([1, 2], 2, 2)'                       [[1, 2, 1, 2], [1, 2, 1, 2]]
'cumsum([1, 2, 3, 4])'                       [1, 3, 6, 10]
'sortrows([[3, 1], [1, 2], [2, 0]])'         [[1, 2], [2, 0], [3, 1]]
'find([0, 3, 0, 5])'                         [2, 4]
'kron([[1, 2], [3, 4]], [[0, 1], [1, 0]])'   [[0, 1, 0, 2], [1, 0, 2, 0], [0, 3, 0, 4], [3, 0, 4, 0]]
'magic(4)'                                   [[16, 2, 3, 13], [5, 11, 10, 8], [9, 7, 6, 12], [4, 14, 15, 1]]
'circshift([1, 2, 3, 4, 5], 2)'              [4, 5, 1, 2, 3]
'unique([3, 1, 2, 3, 1])'                    [3, 1, 2]
'any([0, 0, 1]) && all([1, 1, 0])'           false
```

### Data, statistics and tests

Sums and statistics of a matrix run down its columns (a second argument 2: along the rows).
The tests give what MATLAB gives first: `[h, p]` for `ttest`, `[p, h]` for the tests of ranks
and signs, `p` for the comparisons of groups. A probability that is found by counting every
case is a fraction.

```
'mean([[1, 2], [3, 4]])'                      [2, 3]
'median([3, 1, 4, 1, 5])'                     3
'std([2, 4, 4, 4, 5, 5, 7, 9])'               4sqrt(14)/7
                                                ~ 2.138089935299395077476427847038
'movmean([1, 2, 3, 4, 5], 3)'                 [3/2, 2, 3, 4, 9/2]
'prctile([1, 2, 3, 4, 5], [25, 50, 75])'      [7/4, 3, 17/4]
'corrcoef([1, 2, 3, 4], [2, 4, 5, 9])'        [[1, 11sqrt(130)/130], [11sqrt(130)/130, 1]]
'histcounts([1, 2, 2, 3, 5], 2)'              [3, 2]
'zscore([1, 2, 3])'                           [-1, 0, 1]
'ttest([5.1, 4.9, 5.6, 5.8, 6.0], 5)'         [0, 0.082568296745773799057609961263288]
'ranksum([1, 2, 3], [4, 5, 6])'               [1/10, 0]
'signrank([1.5, -0.3, 2.2, 3.1, -1.2, 4.4, 0.7, 2.9])'
                                              [0.0546875, 0]
'fishertest([[8, 2], [1, 5]])'                [1, 5/143, 20]
'kruskalwallis([[1, 2, 3, 4], [3, 5, 6, 7], [8, 9, 10, 11]])'
                                              0.010882339478324631622855533212248
'kstest([-1.2, -0.5, 0.1, 0.3, 0.8, 1.4, 2.0])'
                                              [0, 0.66928382647137450666248695037868, 0.2541135515627432671796903325249]
'anova2([[1, 2, 3], [2, 4, 5], [3, 5, 9], [4, 6, 8]])'
                                              [0.003723875, 0.0058651509922152968254263305009713]
'getfield(fitlm([1, 2, 3, 4, 5], [2.1, 3.9, 6.2, 7.8, 10.1]), "coefficients")'
                                              [0.05, 1.99]
'cluster(linkage([[0, 0], [1, 0], [0, 3], [5, 5], [6, 5]]), 2)'
                                              [1, 1, 1, 2, 2]
'kmeans([1, 2, 3, 10, 11, 12], 2)'            [[1, 1, 1, 2, 2, 2], [2, 11]]
'normpdf(0)'                                  sqrt(2)/(2sqrt(pi))
                                                ~ 0.39894228040143267793994605993438
'norminv(0.975)'                              1.9599639845400542355245944305206
'rng(0); rand(1, 3)'                          0
                                              [0.81472368639317893634910205946653, 0.90579193707561922455084868488484, 0.12698681629350605515327288230765]
```

The distributions have MATLAB's names (`normpdf`, `tcdf`, `chi2inv`, `binornd`, `poisstat`, ...
and `pdf("Normal", x, mu, sigma)`). After `rng(seed)`, `rand` and `randi` give the numbers
that MATLAB gives after the same call; `randn` and `randperm` are random, but not MATLAB's.

### Matrices taken apart

```
'lu([[4, 3], [6, 3]])'        [[[1, 0], [2/3, 1]], [[6, 3], [0, 1]], [[0, 1], [1, 0]]]
'chol([[4, 2], [2, 3]])'      [[2, 1], [0, sqrt(2)]]
'svd([[3, 0], [0, 4]])'       [4, 3]
'null([[1, 2], [2, 4]])'      [[-2, 1]]
'pinv([[1, 2], [2, 4]])'      [[1/25, 2/25], [2/25, 4/25]]
'expm([[0, 1], [0, 0]])'      [[1, 1], [0, 1]]
'[[4, 1], [1, 3]] \ [1, 2]'   [1/11, 7/11]
'cond([[1, 2], [3, 4]])'      sqrt(15 + sqrt(221))/sqrt(15 - sqrt(221))
                                ~ 14.933034373659252761306004106966
'rank(magic(4))'              3
'gallery("lehmer", 3)'        [[1, 1/2, 1/3], [1/2, 1, 2/3], [1/3, 2/3, 1]]
```

`qr`, `schur`, `hess`, `eig` with vectors, `svd` with its three factors (`singularvaluedecomposition`),
`null`, `orth`, `rref`, `kron`, `expm`, `logm`, `sqrtm`, `funm`, `polyeig`, `pca`, norms and
condition numbers are there; the iterative solvers (`pcg`, `gmres`, `bicgstab`, ...) give the
solution itself.

### Polynomials, interpolation, filters

A polynomial is the list of its coefficients, the highest first.

```
'roots([1, -6, 11, -6])'                      [1, 2, 3]
'polyfit([0, 1, 2, 3], [1, 3, 7, 13], 2)'     [1, 1, 1]
'polyval([1, 2, 3], 2)'                       11
'conv([1, 2], [1, 3])'                        [1, 5, 6]
'deconv([1, 5, 6], [1, 2])'                   [[1, 3], [0, 0, 0]]
'residue([1, 2], [1, 3, 2])'                  [[0, 1], [-2, -1], []]
'interp1([1, 2, 3], [10, 20, 40], 5/2)'       30
'interp1([1, 2, 3, 4], [1, 4, 9, 16], 5/2, "spline")'
                                              25/4
                                                ~ 6.25
'fft([1, 2, 3, 4])'                           [10, -2 + 2i, -2, -2 - 2i]
'filter([1, 1], [1], [1, 2, 3, 4])'           [1, 3, 5, 7]
'trapz([0, 1, 3], [1, 2, 4])'                 15/2
                                                ~ 7.5
'gradient([1, 4, 9, 16])'                     [3, 4, 6, 7]
```

### Texts, patterns and dates

A text stands in double quotes. The patterns are regular expressions as MATLAB writes them
(`\<` and `\>` for the ends of a word, `(?<name>...)`, `$1` in a replacement). A date is a day
number as MATLAB counts it (day 1 is the first of January of the year 0), a date in quotes or
`[year, month, day]`; day numbers are exact, so a half day is `1/2`.

```
'upper("abc")'                                "ABC"
'strrep("hello world", "o", "0")'             "hell0 w0rld"
'strsplit("a,b,c", ",")'                      ["a", "b", "c"]
'strjoin(["a", "b", "c"], "-")'               "a-b-c"
'sprintf("%5.2f and %d", pi, 42)'             " 3.14 and 42"
'num2str(pi)'                                 "3.1416"
'dec2bin(10)'                                 "1010"
'regexp("a12b345c", "\d+", "match")'          ["12", "345"]
'regexp("key=value", "(\w+)=(\w+)", "tokens", "once")'
                                              ["key", "value"]
'regexprep("hello world", "(\w+) (\w+)", "$2 $1")'
                                              "world hello"
'datenum(2026, 11, 20)'                       740306
'datestr(740306.5)'                           "20-Nov-2026 12:00:00"
'datestr("2026-11-20", "dddd, dd mmmm yyyy")'
                                              "Friday, 20 November 2026"
'weekday("2026-11-20", "long")'               "Friday"
'addtodate(datenum(2024, 1, 31), 1, "month")'
                                              739311
'datenum(2026, 12, 25) - datenum("2026-11-20")'
                                              35
```

### Functions as values

`@(x) x^2`, `x -> x^2` and `@sin` are functions that can be given to a command: zeros and
least values, fits, integrals, differential equations step by step, and `cellfun`, `arrayfun`,
`feval`. The numeric answers have the digits that are asked for, not 16.

```
'fzero(@(x) cos(x) - x, 1)'                   0.73908513321516064165531208767387
'fminsearch(@(x, y) (x - 1)^2 + (y + 2)^2, [0, 0])'
                                              [1, -2]
'fmincon(@(x, y) x^2 + y^2, [0, 0], [], [], [1, 1], [1])'
                                              [0.5, 0.5]
'fmincon(@(x, y) 100*(y - x^2)^2 + (1 - x)^2, [0, 0], [], [], [], [], [], [], @(x, y) x^2 + y^2 - 1)'
                                              [0.78641515416842783006654594277742, 0.61769831252339348456755553728674]
'linprog([-1, -1], [[1, 2], [3, 1]], [4, 6])'
                                              [8/5, 6/5]
'quadprog([[2, 0], [0, 2]], [-4, -4], [], [], [], [], [0, 0], [1, 1])'
                                              [1, 1]
'lsqcurvefit(@(a, b, x) a*exp(b*x), [1, 1], [0, 1, 2], [2, 2e, 2e^2])'
                                              [2, 1]
'integral(@(x) exp(-x^2), 0, inf)'            sqrt(pi)/2
                                                ~ 0.88622692545275801364908374167057
'ode45(@(t, y) -y, [0, 1/2, 1], 1)'           [[0, 1], [1/2, 0.60653065971263342360379953499118], [1, 0.36787944117144232159552377016146]]
'cellfun(@(x) 2x, [1, 2, 3])'                 [2, 4, 6]
```

### Records and tables

`struct(name, value, ...)` is a record with named fields and `dictionary(keys, values)` a table
that finds a value by its key. Both are shown as the call that makes them, and the functions
give new ones: nothing is changed in place.

```
's := struct("name", "pi", "value", pi); getfield(s, "value")'
                                              struct("name", "pi", "value", pi)
                                              pi
                                                ~ 3.1415926535897932384626433832795
'setfield(struct("a", 1), "b", x^2)'          struct("a", 1, "b", x^2)
'fieldnames(struct("a", 1, "b", 2))'          ["a", "b"]
'd := dictionary(["a", "b"], [1, 2]); lookup(d, "b")'
                                              dictionary("a", 1, "b", 2)
                                              2
'insert(dictionary("a", 1), "b", 2)'          dictionary("a", 1, "b", 2)
'keys(dictionary([1, 2], ["x", "y"]))'        [1, 2]
```

### Charts of data, files, pictures as matrices

`bar`, `barh`, `pie`, `stairs`, `stem`, `errorbar`, `scatter`, `lineplot`, `loglog`, `semilogx`,
`semilogy`, `contour`, `quiver`, `boxplot`, `spy`, `compass`, `feather`, `polarscatter` and, in
space, `plot3`, `scatter3`, `stem3`, `surf`, `contour3`, `quiver3`, `waterfall` draw lists and
matrices of numbers: with characters in the terminal, and as pictures with `save_png` and
`save_jxl` (all that is chosen for a picture applies, and `title`, `xlabel`, `ylabel`).
`imread` gives the samples of a PNG or JPEG XL file as a matrix (three matrices for colours),
`imwrite` saves a matrix as a picture; `readmatrix`, `writematrix`, `dlmread`, `dlmwrite`,
`csvread`, `csvwrite` read and write numbers in files. Pictures as matrices have `rgb2gray`,
`imresize`, `imrotate`, `imcrop`, `imfilter`, `imhist`, `imbinarize`, `imcomplement`.

```
$ symbolic-math 'bar([3, 1, 4, 1, 5], width = 40, height = 8, title = "the digits of pi")'
the digits of pi
 5.2 │                                ⣤⣤⣤⣤⣤⣤
     │                 ⣀⣀⣀⣀⣀⣀         ⣿⣿⣿⣿⣿⣿
     │                 ⣿⣿⣿⣿⣿⣿         ⣿⣿⣿⣿⣿⣿
     │  ⣶⣶⣶⣶⣶⣶         ⣿⣿⣿⣿⣿⣿         ⣿⣿⣿⣿⣿⣿
     │  ⣿⣿⣿⣿⣿⣿         ⣿⣿⣿⣿⣿⣿         ⣿⣿⣿⣿⣿⣿
     │  ⣿⣿⣿⣿⣿⣿         ⣿⣿⣿⣿⣿⣿         ⣿⣿⣿⣿⣿⣿
     │  ⣿⣿⣿⣿⣿⣿ ⢰⣶⣶⣶⣶⣶⡆ ⣿⣿⣿⣿⣿⣿ ⢰⣶⣶⣶⣶⣶⡆ ⣿⣿⣿⣿⣿⣿
-0.2 │──⠛⠛⠛⠛⠛⠛─⠘⠛⠛⠛⠛⠛⠃─⠛⠛⠛⠛⠛⠛─⠘⠛⠛⠛⠛⠛⠃─⠛⠛⠛⠛⠛⠛──
     └────────────────────────────────────────
      0.4                                  5.6
5 bars
$ symbolic-math 'save_png("bar.png", bar([3, 1, 4, 1, 5]), title = "of pi", xlabel = "place", ylabel = "digit")'
5 bars
saved bar.png: PNG, 1024 x 768 pixels, 8 bits, RGB, level 9, 5461 bytes
$ symbolic-math 'imwrite([[0, 128], [255, 64]], "gray.png"); imread("gray.png")'
saved gray.png: PNG, 2 x 2 pixels, 8 bits, gray, level 9, 84 bytes
[[0, 128], [255, 64]]
$ symbolic-math 'writematrix(magic(3), "m.csv"); readmatrix("m.csv")'
saved m.csv: 3 row(s) of 3 number(s), 18 bytes
[[8, 1, 6], [3, 5, 7], [4, 9, 2]]
$ symbolic-math 'rgb2gray([[[255, 0]], [[0, 255]], [[0, 0]]])'
[[76, 150]]
```

### More special functions and tools for expressions

```
'meijerg([], [], [0], [], 0.5)'               0.60653065971263342360379953499118
'hypergeom([1, 1, 1], [2, 2], -3.5)'          0.61774194259370717002815699860553
'lerchphi(0.5, 2, 3)'                         0.15792421172010004722125056127744
'besselh(0, 1, 2.5)'                          -0.048383776468197996327287778851203 + 0.49807035961523188782747235036209i
"functionalderivative(y'^2/2 - y^2/2, y, x)"
                                              -y - y''
"odetovectorfield(y'' + y = 0, y, x)"         [y', -y]
                                              note: the derivatives of [y, y']
'ztrans(n, n, z)'                             z/(z - 1)^2
'rewrite(sin(x)^2, "cos")'                    1 - cos(x)^2
```

### Pairs of matrices, more statistics, solvers without slopes

`balance`, `qz`, `ordqz`, `gsvd`, `ordschur`, `ordeig`, `rsf2csf` and `cdf2rdf` take pairs of
matrices and Schur forms apart; `runstest`, `ansaribradley`, `lillietest`, `mvncdf`, `cmdscale`,
`procrustes`, `canoncorr`, `nnmf` and the fits of laws to data (`poissfit`, `gamfit`, `wblfit`,
`betafit`, `mle`, `fitdist`, ...) add to the statistics; `bvp4c`, `dde23` and `pdepe` solve
problems with values at both ends, equations that look back and the heat equation, and
`patternsearch`, `particleswarm`, `simulannealbnd`, `ga`, `globalsearch`, `fminimax` and
`fgoalattain` find least values without slopes.

```
'A := [[1, 2], [3, 4]]; B := [[5, 6], [7, 9]]; r := qz(A, B); ordeig(r[[1]], r[[2]])'
                                              [[1, 2], [3, 4]]
                                              [[5, 6], [7, 9]]
                                              [[[-0.43488716303224179073838374858023, 2.9429655768591232259913757393031], [0, 4.598894081064709236796488328344]], [[0.29839376577929616750329705686448, 9.4779468545880752330734853349381], [0, 10.053829349165822280042973847347]], [[-0.9879094023700189160834515073146, -0.15503229569645178814084957963601], [0.15503229569645178814084957963601, -0.9879094023700189160834515073146]], [[-0.79183667940534439563673904642093, -0.61073289836745968326415112182654], [0.61073289836745968326415112182654, -0.79183667940534439563673904642093]]]
                                              [-1.4574271077563381099751019113698, 0.45742710775633810997510191136982]
'lillietest([2.1, 3.4, 1.9, 5.6, 4.4, 3.8, 2.7, 4.1, 3.3, 3.0, 2.2, 4.9])'
                                              [0, 0.9555, 0.10981074145421267330444417253253]
'mvncdf([1, 1], [0, 0], [[1, 1/2], [1/2, 1]])'
                                              0.74520358684674973096290605097485
'gamfit([1, 2, 3, 4, 10])'                    [1.8726098439573354967813211301859, 2.1360562708283689763846163631734]
'bvp4c(@(x, y, z) [z, -y], @(ya, za, yb, zb) [ya, yb - 1], [0, pi/4, pi/2], [0, 0])'
                                              [[0, 0, 1], [pi/4, 0.70710678118654752440084436210485, 0.70710678118654752440084436210485], [pi/2, 1, 0]]
'dde23(@(t, y, z) -z, [1], 1, [0, 1, 2])'     [[0, 1], [1, 0], [2, -0.5]]
'patternsearch(@(x, y) abs(x - 1) + abs(y + 2), [0, 0])'
                                              [1, -2]
```

### Tables, tables with times, values of a few kinds

`table("name", [values], ...)` is a table of named columns, `timetable` one with a time for
every row, `categorical` a list of values of a few kinds. They are shown as tables; the
functions give new ones.

```
't := table("name", ["ann", "bob", "cy"], "age", [31, 25, 40]); sortrows(t, "age")'
                                              table("name", ["ann", "bob", "cy"], "age", [31, 25, 40])
                                              table("name", ["bob", "ann", "cy"], "age", [25, 31, 40])
't := table("g", ["a", "b", "a"], "x", [1, 2, 3]); groupsummary(t, "g", "mean")'
                                              table("g", ["a", "b", "a"], "x", [1, 2, 3])
                                              table("g", ["a", "b"], "GroupCount", [2, 1], "mean_x", [2, 2])
'l := table("id", [1, 2], "x", [10, 20]); r := table("id", [2, 3], "y", ["b", "c"]); outerjoin(l, r)'
                                              table("id", [1, 2], "x", [10, 20])
                                              table("id", [2, 3], "y", ["b", "c"])
                                              table("id", [1, 2, 3], "x", [10, 20, undefined], "y", [undefined, "b", "c"])
'tt := timetable([1, 2, 3, 4], "v", [10, 20, 40, 80]); retime(tt, [1, 3], "sum")'
                                              timetable("Time", [1, 2, 3, 4], "v", [10, 20, 40, 80])
                                              timetable("Time", [1, 3], "v", [30, 120])
'c := categorical(["red", "blue", "red", "green", "red"]); countcats(c)'
                                              categorical(["red", "blue", "red", "green", "red"], ["blue", "green", "red"])
                                              [1, 1, 3]
```

### More charts, expressions as code

`bar3`, `histogram2`, `surfc`, `meshc`, `streamline`, `polarhistogram`, `dendrogram`,
`bubblechart`, `fimplicit3`, `isosurface`, `triplot`, `trisurf`, `pareto`, `fill`, `ribbon`,
`normplot`, `parallelplot`, `swarmchart`, `violinplot`, `histfit`, `gscatter`, `andrewsplot`,
`cdfplot`, `plotmatrix`, `stackedplot`, `biplot` are drawn with characters and saved as
pictures like the other charts. `ccode`, `fortran` and `matlabcode` write an expression as code,
`matlabfunction` makes a function of it, `subexpr` and `cse` find its shared parts and `pretty`
shows it as on paper.

```
$ symbolic-math 'dendrogram(linkage([[0, 0], [1, 0], [5, 0], [6, 1], [10, 10]]), width = 50, height = 10)'
  10.34 │     ⢠⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⢤
        │     ⢸                       ⢸
        │     ⢸                       ⢸
        │     ⢸                       ⢸
        │     ⢸                       ⢸
        │     ⢸              ⡤⠤⠤⠤⠤⠤⠤⠤⠤⠼⠤⠤⠤⠤⠤⠤⠤⠤⠤⡄
        │     ⢸              ⡇                  ⡇
        │     ⢸              ⡇             ⢀⣀⣀⣀⣀⣇⣀⣀⣀⣀⡀
        │     ⢸         ⡏⠉⠉⠉⠉⠉⠉⠉⠉⠉⡇        ⢸         ⡇
-0.9849 │─────⠈─────────⠁─────────⠁────────⠈─────────⠁─────
        └──────────────────────────────────────────────────
         0.4                                            5.6
the tree of 5 leaves, from the left: 5, 1, 2, 3, 4; the last joining at the height 9.849
$ symbolic-math 'ccode(x^2 + sin(y)/3)' 'fortran(x^2 + sin(y)/3)' 'cse([sin(x)*cos(x), sin(x) + cos(x), cos(x)^2])'
"t0 = x*x + sin(y)/3.0;"
"      t0 = x**2 + sin(y)/3.0D0"
[[sigma2 = sin(x), sigma1 = cos(x)], [sigma1*sigma2, sigma1 + sigma2, sigma1^2]]
$ symbolic-math 'pretty((x^2 + 1)/(x - 1) + sqrt(x))'
           2
          x  + 1
sqrt(x) + ------
          x - 1
```

### Systems of control

A system with one input and one output is the ratio of two polynomials in `s` (in `z` when it
is sampled): `tf`, `zpk`, `ss` and `pid` make one, the four operations, `series`, `parallel`
and `feedback` join them. Answers are exact where the input is: `step`, `impulse` and `lsim`
give expressions in `t`. There are `pole`, `zero`, `dcgain`, `damp`, `bode`, `nyquist`, `margin`,
`allmargin`, `hinfnorm`, `h2norm`, `stepinfo`, `c2d`, `d2c`, `rlocus`, `ctrb`, `obsv`, `gram`,
`lyap`, `dlyap`, `care`, `dare`, `lqr`, `dlqr`, `lqe`, `dlqe`, `lqi`, `reg`, `acker`, `place`,
and the charts `bodeplot`, `nyquistplot`, `nicholsplot`, `pzplot`, `rlocusplot`, `stepplot`,
`impulseplot`, `initialplot`.

Systems are values that the signs join: with `s := tf("s")` a formula in `s` is a system, and
so are `tf(...) + tf(...)`, `2*tf(...)`, `1/tf(...)` and whole powers, with numbers and names
among them; `plus`, `minus`, `times`, `mtimes`, `rdivide`, `mrdivide`, `power` and the others
do the same by name. What a numerator and a denominator share cancels. In lists the systems
are joined entry by entry, and a product of matrices of systems is one of systems.

The states of a stable system are weighed by `hsvd`, put into the balanced form by `balreal`,
and the weak ones are taken out by `balred` (at rest, so that the gain at frequency 0 stays,
or cut off); states that the input does not reach or the output does not show go first. `covar` is the spread of the output under white noise, `lyapchol` and `dlyapchol`
give the triangles of the Gramians, `initial` the output of a system left alone from a state,
`gensig` a signal to drive it with, `ss2ss` other states, `d2d` another time between samples,
`ord2` and `filt` systems of the second order and in `1/z`, `sigma` and `nichols` the gain
(and the phase) at frequencies.

```
's := tf("s"); g := 1/(s^2 + 3s + 2); step(g)'
                                              tf([1, 0], [1])
                                              tf([1], [1, 3, 2])
                                              e^(-2t)*(e^t - 1)^2/2
'tf([1], [1, 1]) + 2/tf([1, 3], [1, 2])'      tf([2, 7, 7], [1, 4, 3])
'feedback(tf([1], [1, 1]), 1)'                tf([1], [1, 2])
'margin(tf([4], [1, 3, 3, 1]))'               [2, 27.141630595376226979139373587665, 1.7320508075688772935274463415059, 1.2328187619393802564187234770704]
'c2d(tf([1], [1, 1]), 1/10)'                  tf([1 - 1/e^(1/10)], [1, -1/e^(1/10)], 1/10)
'lqr([[0, 1], [0, 0]], [[0], [1]], [[1, 0], [0, 1]], [[1]])'
                                              [[[1, 1.7320508075688772935274463415059]], [[1.7320508075688772935274463415059, 1], [1, 1.7320508075688772935274463415059]], [-0.86602540378443864676372317075294 - 0.5i, -0.86602540378443864676372317075294 + 0.5i]]
'place([[0, 1], [0, 0]], [[0], [1]], [-1 + i, -1 - i])'
                                              [2, 2]
'hsvd(tf([1, 2], [1, 3, 5, 4]))'              [0.39556142619996876043940750810744, 0.15403860116411174834253601363655, 0.0084771749641429879031285055291039]
'dcgain(balred(tf([1, 2], [1, 3, 5, 4]), 2))'
                                              0.5
'covar(tf([1], [1, 3, 2]), 1)'                [1/12, [[1/6, 0], [0, 1/12]]]
'initial(ss([[0, 1], [-2, -3]], [[0], [1]], [[1, 0]], [[0]]), [1, 0])'
                                              2e^(-t) - e^(-2t)
```

### Filters and spectra

Windows (`hann`, `hamming`, `blackman`, `kaiser`, `chebwin`, ...), the lowpass prototypes and
the filters of Butterworth, Chebyshev, Cauer and Bessel (`butter`, `cheby1`, `cheby2`, `ellip`,
`besself` with their orders `buttord`, ...), filters by windows (`fir1`, `kaiserord`), `sgolay`,
the responses (`freqz`, `freqs`, `grpdelay`, `impz`, `stepz`), `filtfilt`, sections of second
order, spectra (`periodogram`, `pwelch`, `cpsd`, `mscohere`, `tfestimate`, `spectrogram`),
`findpeaks`, `envelope`, linear prediction (`levinson`, `aryule`, `arburg`, `lpc`) and
waveforms. A filter is its pair `[b, a]`, which every command takes as one value.

Filters of linear phase are also made with the least square error (`firls`), with ripples of
one size (`firpm` or `remez`: the exchange of Remez, with the greatest errors sought on the
whole bands, not on a grid) and through given gains (`fir2`). `resample` changes the rate by
a ratio, `phasez`, `phasedelay` and `zerophase` are the phase without jumps, its delay and the
real response, `finddelay` and `dtw` lay two signals along each other, `residuez` takes a
ratio in `1/z` apart, `tf2latc`, `latc2tf` and `latcfilt` are the lattices, `fftfilt` and
`filtic` filter by transforms and from a state.

```
'butter(2, 0.3)'                              [[0.13110643991662595261757758343356, 0.26221287983325190523515516686712, 0.13110643991662595261757758343356], [1, -0.74778917825850341013599412216864, 0.27221493792500722060630445590287]]
'freqz(butter(3, 0.4), 3)'                    [[1, -0.21005729606506965947802060512306 - 0.86874640169822398370743979245151i, -0.056166563213096094569655954231284 + 0.047574864502733976185738788522147i], [0, 1.0471975511965977461542144610932, 2.0943951023931954923084289221863]]
'filtfilt(butter(2, 0.3), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])'
                                              [1.0063625254535161969171634115367, 2.00516952495721321678757610403, 3.00234167168594773687911336378, 4.0007282808866946751750414760592, 5.0001993519080990591169947487595, 5.999341269108394711220813597023, 6.9967052832026297237267613257207, 7.9924062890931669964367006107933, 8.9907279576137653315456385413321, 10.00230214168777727040206240452]
'buttord(0.2, 0.3, 1, 40)'                    [12, 0.21270871824929540864901253127966]
'findpeaks([1, 3, 2, 5, 4, 6, 1])'            [[3, 5, 6], [2, 4, 6]]
'firpm(6, [0, 0.4, 0.6, 1], [1, 1, 0, 0])'    [-0.11959674888504780243751827482553, 0, 0.3131083535250412376370471014008, 0.5, 0.3131083535250412376370471014008, 0, -0.11959674888504780243751827482553]
'resample([1, 2, 3, 4, 5, 6], 3, 2)'          [1.0006061735537776941809619013042, 1.8079145190652872091275328461087, 2.1680722384446313211004223855346, 3.0018185206613330825428857039125, 3.940992689282657130100073137418, 3.9656716408048488224405763555171, 5.0030308677688884709048095065208, 6.5681064576906517785429079645452, 4.240290707826440579514136673378]
'finddelay([1, 2, 3, 0, 0], [0, 0, 1, 2, 3])'
                                              2
'tf2latc([1, 1/2, 1/4])'                      [0.4, 0.25]
```

### The earth

Ways on a sphere and on an ellipsoid, with 32 digits or more: `distance`, `azimuth` and
`reckon` along great circles, along the geodesics of an ellipsoid (`"wgs84"`, `"grs80"`, ...,
`[half axis, eccentricity]` or the radius of a sphere) and along lines of one course (`"rh"`);
`geoddistance`, `geodreckon`, `geodarea` and `areaint` (polygons whose sides are geodesics);
tracks, circles and their meeting points; the frames of space (`geodetic2ecef`, `ecef2enu`,
`geodetic2aer`, ...); latitudes of other kinds, radii, ellipsoids; exact units of lengths and
angles; and maps (`projfwd`, `projinv`: Mercator, transverse Mercator and UTM, Lambert's and
Albers' cones, stereographic, azimuthal, Mollweide, ...). Angles are degrees unless `"radians"`
is said.

```
'distance(52.2297, 21.0122, 40.7128, -74.006, "wgs84")'
                                              6872969.7520297344477883147344393
'azimuth(52.2297, 21.0122, 40.7128, -74.006, "wgs84")'
                                              300.93658201393419650653583771645
'geoddistance(0, 0, 0.5, 179.7)'              [19944127.420750460182045203151119, 15.556882793489957368987002286525, 164.44251389085553264795970647389]
'reckon(52.2297, 21.0122, 6872969.752, 300.9366, "wgs84")'
                                              [40.712811019967953920762765847693, -74.006015042114613044316679499629]
'areaint([0, 90, 0], [0, 0, 90])'             0.125
'geodetic2ecef(48.8562, 2.3508, 67.4)'        [4200996.7895534321572836309914208, 172460.32153737180955873582614375, 4780102.8308752110502429869903845]
'deg2km(1)' 'nm2sm(1)' 'wrapto180(1000sqrt(2))'
                                              6371pi/180
                                                ~ 111.19492664455873734580833886041
                                              1822831/1584000
                                                ~ 1.1507771464646464646464646464646
                                              1000sqrt(2) - 1440
                                                ~ -25.786437626904951198311275790302
'projfwd("utm", 52.52, 13.405)' 'deg2utm(52.52, 13.405)'
                                              [391779.25925274886473760192753298, 5820072.1592110269712351471238236]
                                              [391779.25925274886473760192753298, 5820072.1592110269712351471238236, "33U"]
```

### Networks of neurons

The functions of units (`relu`, `sigmoid`, `gelu`, `softmax`, ...) and the losses (`mse`,
`crossentropy`, `huber`, ...) are exact and take names, so that they have slopes and
integrals; the operations of layers are `fullyconnect`, `dlconv`, `maxpool`, `avgpool`,
`batchnorm`, `layernorm`, `embed`, `lstm`, `gru`. A network of dense layers is a plain value, a
list of layers `[weights, biases, "function"]`: `feedforwardnet` and `patternnet` make one with
weights that are the same on every machine, `predict` and `classify` use it, `trainnet` finds
the weights that make the loss least, and `perceptron`, `newlind` and `newrbe` teach one layer
exactly.

```
'sigmoid(x)' 'diff(sigmoid(x), x)' 'relu([-2, 3/2, x])'
                                              e^x/(e^x + 1)
                                              e^x/(e^x + 1)^2
                                              [0, 3/2, x/2 + abs(x)/2]
'crossentropy([1/2, 1/4, 1/4], [0, 1, 0])'    2ln(2)
                                                ~ 1.3862943611198906188344642429164
'dlconv([1, 2, 3, 4, 5], [1, 0, -1])'         [-2, -2, -2]
'predict([[[[1, 2]], [0], "tanh"]], [x, y])'
                                              tanh(x + 2y)
'n := trainnet(feedforwardnet([2, 3, 1]), [[0, 0], [0, 1], [1, 0], [1, 1]], [0, 1, 1, 0]); predict(n, [[0, 0], [0, 1], [1, 0], [1, 1]])'
                                              [[[[-1.5593111976748562181481237054619, 2.3123583995979380205359152887447], [1.8737670968965111728250142469164, 1.4359779290051702595716891081343], [1.0925094885194510464567038796402, -0.43961668273364131771864803575]], [1.4400756727768834308185805004257, 0.39882989693949209307197343570631, -0.280099398568542311014202295086], "tanh"], [[[-1.2604978439472344973318994107689, 1.3066141023723713593881953271341, -1.1231219191572280603582219984132]], [0.32477633327879649507530303038942], "purelin"]]
                                              [-6.143284952071892301390187465889e-23, 0.99999999999999999999969609747215, 0.99999999999999999999987162766187, -6.6055715083967000056425267005849e-23]
'perceptron([[0, 0], [0, 1], [1, 0], [1, 1]], [0, 0, 0, 1])'
                                              [[[[2, 1]], [-3], "hardlim"]]
```

### For users of MATLAB

What is the same thing under another form, and what differs on purpose:

- `x = 3` is an equation; `x := 3` gives a name a value. A name of several letters before a
  bracket is a function (`x(1)` multiplies); the entry of a list is `list[[i]]`.
- A function with several outputs gives a list: `lu` gives `[L, U, P]`, `pca` gives
  `[directions, scores, variances]`, `ttest` gives `[h, p]`, `kmeans` gives
  `[groups, means]`, `fitlm` gives a record.
- `0^0` is `undefined` (MATLAB: 1). `eps` is a free name; `sqrt(-1)` and `i` are exact.
- `reshape` fills along the rows, as lists are written; `unique` keeps the order of first
  appearance (`"sorted"` sorts); `mode` gives every most frequent value; `length` of a matrix
  is the number of its rows; `range` is the range of numbers `a..b` (the spread is `peak2peak`).
- `svd` gives the singular values; the factors are `singularvaluedecomposition`. `gammainc`,
  `sinc`, `legendre`, `dilog`, `dot`, `normalize` and `quantile` keep the meanings that this program gave
  them before (see `--functions`).
- `sprintf("%.2f", 9.995)` is `"10.00"`: decimals are exact and halves round away from zero.
- `lgamma` of a negative number is `ln|gamma|`; `tostring` shows the exact form and `num2str`
  the short one of MATLAB.
- `ode45` gives rows `[t, y]` at the times asked for, worked out with 32 digits or more;
  `quadprog` needs a matrix without negative directions; `fmincon` goes down from its start
  and says so when the function has no least value there (MATLAB stops at a point near the
  start: `-x*y*z` with `0 <= x + 2y + 2z <= 72` has none, from `[1, 1, 1]` the point
  `[24, 12, 12]` of MATLAB's manual is found).
- `regexp` gives the whole match as the token when the pattern has no brackets, and `""` for a
  bracket that matched nothing; `"names"` gives pairs `[name, text]`.
- A date number is exact: `datevec` has no rounding of seconds. Two-digit years are 1950 to
  2049. A year after the 29th of February is the 28th.
- `cluster(steps, n)` numbers the groups as their points first appear; `kmeans` starts from
  the points farthest apart, not at random. `kstest` counts the exact law up to 1000 values;
  `jbtest` uses the chi-square law (MATLAB: a table for small samples). `chi2gof` takes
  counts and expected counts.
- `imresize` is bilinear or `"nearest"` without smoothing first; `imrotate` is exact for
  quarter turns.
- `sparse` and `full` keep every entry: matrices are not stored by their places.
- `distance` of two points is their distance in space; with four numbers it is the way between
  two places of the earth, and `azimuth` gives its course (MATLAB: two outputs of `distance`).
  At a pole a course is counted from the meridian of the longitude that is given, so that
  `reckon` leads back; `departure` takes the longitudes as they are given; a statute mile is
  the survey mile of 6336000/3937 m. `areaint` is the exact area of the polygon of geodesics.
- A network is a list of layers, rows of data are observations, and `feedforwardnet` takes
  the numbers of inputs and outputs too: `feedforwardnet([2, 3, 1])`. `trainnet` goes down to a
  least value with every digit of the session (BFGS); it does not stop early.

### What is not there

Of the toolboxes for control, signal design, maps and deep learning their cores are here, not
all of them: systems with several inputs and outputs (and what is made of them: `kalman`,
`lqg`, `append`, `connect`), delays inside systems, the filters of `firls` and `firpm` with
taps of the other sign from the end (Hilbert transformers, differentiators), MATLAB's own
lowpass in `resample` (the one of SciPy's `resample_poly` is used), raster maps and geoids,
convolutional networks that are trained and stochastic training are missing. There is
no Simulink, no image processing beyond the functions above, no cell array as a type of its own
(a list holds unlike things), no `latex`, no `newUnit`, no MATLAB streams of `randn` and
`randperm`, and no graphical user interface, object or file of MATLAB's own formats.

## Tetration

`b^^h` or `tet(b, h)` is tetration, `slog(b, z)` (or `ate`) is the height `h` with `b^^h = z`,
and `ssrt(z, h)` (or `sroot`) is the base `b` with `b^^h = z`. Whole heights are towers of
powers and are calculated here. Every other value is asked of the `tet` program, which must be
on PATH; it is run as `tet -q [--slog|--sroot] <digits> <re> <im> <re> <im>` with the digits of
the session, so the answer has at least 32 digits, or more when more are asked for.

```
$ symbolic-math '2^^4' '2^^x=65536' 'x^^2=27' 'slog(2, 65536)'
65536
x = 4
x = 3
4
$ symbolic-math '2^^0.5' '2^^x=100' 'ssrt(100, 2.5)'
1.4587818160364217006839716610386
x = slog(2, 100)
  ~ 3.4909047106704682566600724383475
2.6205499956482035192909997276562
$ symbolic-math -d 50 '2^^0.5'
1.4587818160364217006839716610385871352966066053309
$ symbolic-math 'sqrt(2)^^inf' 'i^^inf' 'x^^inf = 2'
2
2i*lambertw(-i*pi/2)/pi
  ~ 0.43828293672703211162697516355126 + 0.360592471871385485952940526906i
x = sqrt(2)
  ~ 1.4142135623730950488016887242097
```

A tower without end, `b^^inf`, settles for the bases from `e^(-e)` to `e^(1/e)` (and for the
complex ones with `|W(-ln b)| < 1`) at `-W(-ln b)/ln b`, and the equation `x^^inf = c` has the
solution `c^(1/c)` when the tower of that base settles at `c`.

When `tet` cannot be run, a request for a numeric value is an error; an exact answer is still
printed, with a note. A drawing of tetration is then an error that says so, at once.

Every drawing takes tetration: `plot`, `plot3d`, `densityplot`, `complexplot`, `plot4d` and
`view4d` ask `tet` for the values at all their places before they draw them, as many runs of the
program at a time as the machine has processors (the values are the same as one by one; a
drawing of `2^^x` and `e^^x` is made nine times as fast on 16 processors). Bases from `e^(-e)`
to `e^(1/e)` take hundredths of a second for a value, the others about a second.

```
$ symbolic-math 'plot(sqrt(2)^^x, x, -1, 3, 40, 10)'
1.761 │          │                 ⣀⣀⣠⠤⠤⠴⠒⠒⠒⠚⠉⠉
      │          │         ⣀⣠⠤⠴⠒⠚⠉⠉⠁
      │          │    ⣀⡤⠔⠚⠉⠁
      │          │⣀⠤⠚⠉
      │        ⣀⠴⠋⠁
      │      ⡠⠞⠁ │
      │    ⡠⠊    │
      │  ⢠⠞⠁     │
      │ ⡰⠃       │
    0 │⡼⠁        │
      └────────────────────────────────────────
       -1                                     3
$ symbolic-math 'complexplot(sqrt(2)^^z, z, -1 - 2i, 3 + 2i, width = 24, height = 8)'
 2.1 │⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶
     │⡶⡶⡶⡶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶
     │⡶⡶⡶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⡶
     │⡶⡶⠶⠶⠴⠴⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⡶
     │⡶⡶⠶⠶⠴⠴⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⡶
     │⡶⡶⡶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⡶
     │⡶⡶⡶⡶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶
-2.1 │⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶
     └────────────────────────
      -2.15               4.15
the colour is the argument of the value: red for the positive numbers, then yellow, green, cyan for the negative ones, blue, magenta; the lightness is its absolute value: black is 0, the full colour 1, white the infinitely large
$ symbolic-math 'save_jxl("tetration.jxl", plot4d(sqrt(2)^^z, z, 2), width = 12, height = 12, frames = 4)'
plot4d of (sqrt(2))^^z
the width: the real part of z from -2 to 2; the height: the imaginary part of z from -2 to 2, upwards; the frames: the real part of f(z) from -2.247 to 2.021; the colours: the imaginary part of f(z) from -3.866 to 3.866 (512 colours of the palette rainbow, the least value first)
164 voxels of the box of 12 x 12 x 4 belong to the surface (28.47 %)
saved tetration.jxl: JPEG XL, 12 x 12 pixels, 4 frames, 16 a second, 8 bits, RGB + alpha of 1 bit, lossless, effort 10, 513 bytes
```

## Syntax

- **Numbers**: `42`, `3.14`, `1e-3`, `0x1F`, `0b101`, `0o17`, `2+3i`, `¾`; in a base:
  `16#ff.8`, `2#0.(01)`, `1000#001#234`. Decimals are exact until a numeric value is needed.
- **Texts**: `"walk.png"`, for the names of files and for numbers as they are written.
- **Operators**: `+ - * /`, `^` (right to left: `2^3^2` is `2^9`), `^^` (binds tighter than
  `^`), `!` and `!!` (factorials), `%` or `mod` (remainder), `'` (derivative: `f'`, `f''`), `°`
  (degrees), `|x|` (absolute value), `±` or `+/-` (both values: `x = 3 +/- 1`).
- **Multiplication** may be left out: `2x`, `2 pi r`, `(x+1)(x-1)`, `sin(x)cos(x)`. A single
  letter in front of a bracket multiplies (`x(x+1)`), a longer name must be a function.
- **Relations**: `=`, `!=`, `<`, `<=`, `>`, `>=`; `x -> a` in `limit`, `0+` and `0-` for one side.
  `and`, `or`, `not` (`&&`, `∧`, `∨`, `¬`) join them.
- **MATLAB's forms**: `a:b` and `a:step:b` (ranges), `.*`, `./`, `.\`, `.^` (entry by entry),
  `A \ b` (the solution of `A*x = b`), `@(x) x^2` and `@sin` (functions), `~=`, `~x`, `&`, `|`
  between relations, `&&`, `||`.
- **Lists**: `[1, 2, 3]`, matrices `[1, 2; 3, 4]`, an entry `list[[i]]`. An equation between
  two lists is a system; a list on one side of an equation stands for each of its values.
- **Functions as arguments**: `x -> x^2`, `(a, b) -> a + b`, or a name like `sin`.
- **Iterates**: `f[t](x)` is `f` applied `t` times.
- **Unicode**: `π`, `∞`, `√2`, `∛x`, `x²`, `x₁`, `≤ ≥ ≠`, `× · ÷ −`, `∑ ∏ ∫ ∂ ∇`, `⌊x⌋ ⌈x⌉ ‖v‖`,
  `↑↑` for `^^`, `∈` with `ℝ ℤ ℕ`, Greek letters (`α` is `alpha`). Every Unicode form has an ASCII
  one.

```
'x² + 2x + 1 = 0'     x = -1 (multiplicity 2)
'√2 · √8'             4
'∑(1/k², k, 1, ∞)'    pi^2/6
                        ~ 1.644934066848226436472415166646
'∫(x², x, 0, 1)'      1/3
                        ~ 0.33333333333333333333333333333333
'e^(iπ) + 1'          0
'2↑↑3'                16
'90°'                 pi/2
                        ~ 1.5707963267948966192313216916398
'½ + ⅓'               5/6
                        ~ 0.83333333333333333333333333333333
'sin²(x) + cos²(x)'   1
```

## Names

`symbolic-math --functions` and `symbolic-math --constants` print these lists with every name;
`symbolic-math --units` prints the units.
Names of constants are accepted in any letter case; a name in brackets is another spelling.

- **Elementary**: `sqrt` (`√`), `cbrt` (`∛`), `root`, `sqr` (`square`), `pow` (`powc`, `powf`, `powi`, `powu`), `exp`, `exp2`, `expf`, `ln` (`㏑`), `log` (`㏒`), `log2` (`lb`), `log10` (`lg`), `sin`, `cos`, `tan` (`tg`), `cot` (`ctg`, `cotan`), `sec`, `csc` (`cosec`), `asin` (`arcsin`), `acos` (`arccos`), `atan` (`arctan`, `arctg`), `acot` (`arccot`, `arcctg`), `asec` (`arcsec`), `acsc` (`arccsc`), `atan2`, `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`, `asinh` (`arcsinh`, `arsinh`), `acosh` (`arccosh`, `arcosh`), `atanh` (`arctanh`, `artanh`), `acoth` (`arccoth`, `arcoth`), `asech` (`arcsech`, `arsech`), `acsch` (`arccsch`, `arcsch`).
- **Complex numbers**: `abs` (`mag`, `polar_r`, `radius`), `mod`, `norm` (`‖`), `abs2` (`mag2`, `norm_sqr`, `normsq`), `l1_norm` (`l1`, `manhattan`, `taxicab`), `arg` (`phase`, `polar_theta`, `theta`), `to_polar`, `re` (`real`, `ℜ`), `im` (`imag`, `ℑ`), `conj` (`conjugate`), `inv` (`recip`, `inverse`, `finv`), `sign` (`sgn`, `signum`), `cis`, `scale`, `unscale`, `fdiv`, `complex` (`new`), `rect` (`rectform`), `polar` (`from_polar`, `polarform`), `floor` (`⌊`), `ceil` (`ceiling`, `⌈`), `round`, `trunc` (`fix`), `frac` (`fract`), `minabs`, `maxabs`, `avg` (`mean`, `average`), `min`, `max`, `hypot`, `agm`, `is_nan` (`isnan`), `is_infinite` (`isinf`, `isinfinite`), `is_finite` (`isfinite`), `is_normal` (`isnormal`).
- **Special functions**: `gamma` (`tgamma`, `Γ`), `lgamma` (`loggamma`, `lngamma`, `gammaln`), `factorial` (`fact`), `factorial2` (`doublefactorial`), `binomial` (`choose`), `beta`, `digamma` (`psi`, `ψ`), `polygamma`, `harmonic`, `zeta` (`hurwitzzeta`, `ζ`), `erf`, `erfc`, `erfi`, `ei`, `li` (`logint`), `li2` (`dilog`), `si` (`sinint`), `ci` (`cosint`), `shi` (`sinhint`), `chi` (`coshint`), `lambertw` (`w`, `productlog`), `uppergamma` (`gammainc`, `incgamma`, `igamma`), `lowergamma`, `gammaq` (`gammaregularized`), `gammap`, `expint` (`expintegrale`), `betainc`, `betaregularized`, `erfinv` (`inverseerf`), `erfcinv` (`inverseerfc`), `fresnels`, `fresnelc`, `sinc`, `besselj`, `bessely`, `besseli`, `besselk`, `hankel1` (`hankelh1`), `hankel2` (`hankelh2`), `sphericalbesselj`, `sphericalbessely`, `airyai`, `airybi`, `airyaiprime`, `airybiprime`, `struveh`, `struvel`, `hyp0f1` (`hypergeometric0f1`), `hyp1f1` (`hypergeometric1f1`, `kummerm`), `hyperu` (`hypergeometricu`, `kummeru`), `hyp2f1` (`hypergeometric2f1`), `hypergeometric` (`hypergeometricpfq`, `pfq`, `hypergeom`), `polylog`, `dirichleteta`, `elliptick` (`ellipk`), `elliptice` (`ellipe`), `ellipticf` (`ellipf`), `ellipticpi` (`ellippi`), `ellipticnome` (`nome`), `jacobisn`, `jacobicn`, `jacobidn`, `elliptictheta` (`jtheta`), `legendrep`, `legendreq`, `chebyshevt`, `chebyshevu`, `hermiteh` (`hermite`), `laguerrel` (`laguerre`), `gegenbauerc` (`gegenbauer`), `jacobip`, `sphericalharmonicy` (`sphericalharmonic`, `ylm`), `heaviside` (`heavisidetheta`), `unitstep`, `diracdelta` (`dirac`).
- **Tetration**: `tet`, `slog` (`ate`), `ssrt` (`sroot`).
- **Algebra**: `simplify`, `expand`, `factor`, `collect`, `together` (`simplifyfraction`), `cancel`, `apart` (`partfrac`), `numer` (`numerator`), `denom` (`denominator`), `subst` (`substitute`, `replace`, `subs`), `coeff` (`coefficient`), `degree` (`polynomialdegree`), `quo` (`quotient`), `rem` (`remainder`), `gcd`, `lcm`, `resultant`, `discriminant`, `roots`, `trigexpand` (`expand_trig`), `trigreduce`, `rewrite_exp` (`toexp`), `N` (`evalf`, `approx`, `numeric`, `vpa`), `rationalize`, `identify`.
- **Solving**: `solve`, `qsolve` (`quatsolve`), `nsolve` (`vpasolve`), `dsolve`, `rsolve`.
- **Calculus**: `diff` (`derivative`, `∂`), `D`, `integrate` (`int`, `integral`, `∫`), `nintegrate` (`nint`, `vpaintegral`).
- **Areas volumes and regions**: `area`, `volume`, `perimeter` (`circumference`), `surfacearea`, `centroid`, `arclength`, `revolutionvolume`, `revolutionarea`, `rectangle`, `disk` (`disc`), `annulus`, `sector`, `ellipse`, `triangle`, `polygon`, `box` (`cuboid`), `ball`, `shell`, `ellipsoid`, `simplex`, `tetrahedron`, `cylinder`, `cone`.
- **Limits series and sums**: `limit` (`lim`), `series` (`taylor`), `sum` (`summation`, `∑`, `symsum`), `product` (`prod`, `∏`, `symprod`), `grad` (`gradient`, `∇`), `divergence`, `curl`, `laplacian`, `jacobian`, `hessian`, `minimize`, `maximize`.
- **Linear algebra**: `det` (`determinant`), `transpose`, `trace` (`tr`), `rank`, `rref`, `nullspace` (`kernel`, `null`), `eigenvalues` (`eigvals`, `eig`), `eigenvectors` (`eigvecs`), `charpoly`, `dot`, `cross`, `identity` (`eye`), `linsolve`.
- **Complex parts functional equations and iterates**: `complexexpand`, `funsolve` (`functionalsolve`), `iterate` (`fiterate`).
- **Tensors**: `dimensions` (`dims`, `shape`), `tensorrank` (`arraydepth`), `flatten`, `reshape`, `outer` (`tensorproduct`), `contract` (`tensorcontract`), `inner`, `kron` (`kroneckerproduct`), `levicivita`, `signature`, `kroneckerdelta`, `symmetrize`, `antisymmetrize`, `wedge`, `hodge` (`hodgestar`), `christoffel`, `riemann`, `ricci`, `ricciscalar`, `einstein`, `kretschmann`, `geodesics`, `covariant`, `raiseindex`, `lowerindex`.
- **Geometry**: `line`, `plane`, `circle`, `sphere`, `distance`, `angle`, `projection`, `intersection` (`intersect`), `equation`, `midpoint`, `normalize` (`unit`), `triple`, `collinear`, `coplanar`, `circumcircle` (`circumsphere`), `incircle`, `solidangle`, `ballvolume`, `spherearea`, `rotationmatrix`, `rotationquat`, `rotate`, `reflect`, `topolar` (`tospherical`), `frompolar` (`fromspherical`), `regularpolygon`, `polytope`.
- **Functions of matrices and decompositions**: `adjugate` (`adj`, `adjoint`), `matrixexp` (`expm`), `matrixpower`, `matrixsqrt` (`sqrtm`), `matrixlog` (`logm`), `matrixfunction`, `gramschmidt` (`orthogonalize`), `cholesky`, `qr`, `pinv` (`pseudoinverse`), `singularvalues` (`svd`), `diag`, `zeros`, `ones`, `hilbert` (`hilb`), `vandermonde`.
- **Quaternions and domains**: `quat` (`quaternion`), `domain`.
- **Number theory**: `isprime` (`is_prime`, `primeq`), `iscomposite` (`compositeq`), `isprimepower` (`primepowerq`), `isperfect` (`perfectnumberq`), `iscoprime` (`coprime`, `coprimeq`), `primes`, `primefactors`, `ifactors` (`factorinteger`), `primorial`, `carmichael` (`carmichaellambda`), `liouville` (`liouvillelambda`), `mangoldt` (`mangoldtlambda`), `legendre` (`legendresymbol`), `farey` (`fareysequence`), `squaresr`, `partitionsq`, `eulerian`, `superfactorial`, `hyperfactorial`, `pell`, `egyptian`, `primitiveroots` (`primitiverootlist`), `sqrtmodall` (`sqrtmods`), `pochhammer` (`rising`, `risingfactorial`), `fallingfactorial` (`falling`, `factorialpower`), `nextprime`, `prevprime`, `prime` (`nthprime`), `primepi`, `divisors`, `totient` (`eulerphi`), `sigma` (`divisorsigma`), `mobius` (`moebius`), `fibonacci` (`fib`), `lucas`, `bernoulli`, `eulernumber` (`euler`), `catalannumber`, `powmod` (`powermod`), `modinv`, `jacobi` (`jacobisymbol`), `isqrt`, `cfrac`, `base`, `frombase`, `base_conv` (`baseconv`, `convbase`, `base_convert`, `baseconvert`, `convert_base`), `tobase` (`inbase`, `baseform`), `tofraction` (`basefraction`), `digits` (`realdigits`, `digitlist`), `period` (`periodlength`, `repetend`), `digitcounts` (`digitcount`), `digitsum`, `issquare`, `issquarefree`, `perfectpower`, `primenu`, `primeomega`, `radical`, `kronecker`, `sqrtmod`, `ord` (`multiplicativeorder`), `primitiveroot`, `discretelog` (`dlog`), `crt`, `egcd` (`gcdex`), `iroot`, `sumoftwosquares`, `bell`, `stirling1`, `stirling2`, `partitions`, `derangements` (`subfactorial`), `multinomial`, `convergents`.
- **Lists**: `range`, `table`, `length`, `first`, `last`, `rest`, `most`, `part`, `take`, `drop`, `reverse`, `sort`, `sortby`, `rotateleft`, `rotateright`, `append`, `prepend`, `join` (`concat`), `insert`, `delete`, `union`, `complement` (`setminus`, `setdiff`), `unique` (`deleteduplicates`), `contains` (`memberq`), `count`, `position`, `total`, `accumulate` (`cumsum`), `differences`, `ratios`, `tally`, `zip`, `permutations`, `subsets`, `tuples`, `cartesianproduct`, `partition` (`chunks`), `map` (`arrayfun`, `cellfun`), `select` (`filter`), `reject`, `countif`, `anytrue`, `alltrue`, `findfirst`, `fold` (`reduce`), `foldlist`, `nestlist`, `nest`, `fixedpoint`, `apply`, `array`.
- **Programs and logic**: `if`, `piecewise`, `while`, `do`, `for`, `block`, `and`, `or`, `not`, `xor`, `implies`, `islist` (`listq`), `isnumber` (`numberq`), `isinteger` (`integerq`), `isrational`, `iseven` (`evenq`), `isodd` (`oddq`), `isreal`, `ispositive`, `isnegative`, `isnonnegative`.
- **Statistics**: `median`, `mode` (`commonest`), `variance` (`var`), `stddev` (`standarddeviation`, `std`), `pvariance`, `pstddev`, `covariance`, `correlation` (`corr`), `quantile` (`inversecdf`, `icdf`), `quartiles`, `iqr` (`interquartilerange`), `moment`, `centralmoment`, `skewness`, `kurtosis`, `geometricmean` (`geomean`), `harmonicmean` (`harmmean`), `rms` (`rootmeansquare`), `meandeviation`.
- **Probability**: `pdf` (`pmf`, `probabilitydensity`), `cdf`, `mgf` (`momentgeneratingfunction`), `expectation` (`expectedvalue`), `probability`, `normaldist` (`normaldistribution`), `uniformdist` (`uniformdistribution`), `exponentialdist` (`exponentialdistribution`), `gammadist` (`gammadistribution`), `chisquaredist` (`chisquaredistribution`), `studenttdist` (`studenttdistribution`), `betadist` (`betadistribution`), `cauchydist` (`cauchydistribution`), `lognormaldist` (`lognormaldistribution`), `weibulldist` (`weibulldistribution`), `laplacedist` (`laplacedistribution`), `rayleighdist` (`rayleighdistribution`), `logisticdist` (`logisticdistribution`), `paretodist` (`paretodistribution`), `fdist` (`fratiodistribution`), `binomialdist` (`binomialdistribution`), `poissondist` (`poissondistribution`), `geometricdist` (`geometricdistribution`), `bernoullidist` (`bernoullidistribution`), `discreteuniformdist` (`discreteuniformdistribution`), `negativebinomialdist` (`negativebinomialdistribution`), `hypergeometricdist` (`hypergeometricdistribution`).
- **Numeric solutions lattices fits and equations in whole numbers**: `ndsolve`, `latticereduce` (`lll`), `findrelation` (`findintegerrelation`, `pslq`), `minpoly` (`minimalpolynomial`), `interpolate` (`interpolatingpolynomial`), `fit`, `leastsquares`, `criticalpoints` (`stationarypoints`), `modsolve` (`solvemod`), `diophantine`.
- **Transforms residues and approximants**: `laplace` (`laplacetransform`), `invlaplace` (`inverselaplace`, `ilaplace`, `inverselaplacetransform`), `fourier` (`fouriertransform`), `invfourier` (`inversefourier`, `ifourier`, `inversefouriertransform`), `fourierseries`, `ztransform` (`ztrans`), `invztransform` (`inverseztransform`, `iztransform`, `iztrans`), `residue`, `pade` (`padeapproximant`), `seriescoeff` (`seriescoefficient`).
- **Graphs**: `vertexlist` (`vertices`), `edgelist`, `vertexcount` (`numnodes`), `edgecount` (`numedges`), `adjacencymatrix` (`adjacency`), `weightmatrix` (`weightedadjacencymatrix`), `incidencematrix` (`incidence`), `laplacianmatrix` (`kirchhoffmatrix`), `adjacencygraph`, `degrees` (`degreesequence`, `vertexdegrees`), `indegrees` (`indegree`), `outdegrees` (`outdegree`), `vertexdegree`, `neighbors` (`neighbours`, `adjacentvertices`), `components` (`connectedcomponents`, `conncomp`), `strongcomponents` (`stronglyconnectedcomponents`), `isconnected` (`connectedq`), `shortestpath` (`findshortestpath`), `graphdistance`, `distancematrix` (`graphdistancematrix`, `distances`), `eccentricity` (`vertexeccentricity`), `diameter` (`graphdiameter`), `graphradius`, `graphcenter` (`graphcentre`), `girth`, `spanningtree` (`minimumspanningtree`, `mst`, `minspantree`), `spanningtreecount` (`treecount`), `isbipartite` (`bipartiteq`), `bipartition`, `istree` (`treeq`), `isacyclic` (`acyclicq`), `findcycle`, `topologicalsort` (`toposort`), `chromaticpolynomial`, `chromaticnumber`, `graphcoloring` (`vertexcoloring`), `iseulerian` (`eulerianq`), `eulerianpath` (`euleriancycle`, `eulertour`), `ishamiltonian` (`hamiltonianq`), `hamiltoniancycle` (`findhamiltoniancycle`), `hamiltonianpath`, `maxflow` (`maximumflow`), `maxmatching` (`maximummatching`), `maxclique` (`findclique`), `cliquenumber`, `maxindependentset` (`independentset`), `independencenumber`, `vertexcover` (`minvertexcover`), `complementgraph` (`graphcomplement`), `linegraph`, `subgraph` (`inducedsubgraph`), `isisomorphic` (`isomorphicq`, `graphisomorphic`), `pagerank`, `completegraph`, `cyclegraph`, `pathgraph`, `stargraph`, `wheelgraph`, `gridgraph`, `hypercubegraph`, `petersengraph`, `completebipartite` (`completebipartitegraph`).
- **Plots**: `plot` (`fplot`, `ezplot`), `parametricplot`, `polarplot`, `listplot` (`pointplot`, `scatterplot`), `histogram` (`hist`), `implicitplot` (`contourplot`, `fimplicit`).
- **Pictures**: `walk` (`digitwalk`, `numberwalk`), `walk3d` (`digitwalk3d`, `numberwalk3d`), `primewalk` (`primes_walk`, `prime_walk`, `primeswalk`), `primewalk3d` (`primes_walk3d`, `prime_walk3d`, `primeswalk3d`, `primes3d_xyz`, `primes3d`), `primemodwalk` (`primes_modwalk`, `prime_modwalk`, `primesmodwalk`, `primeresiduewalk`), `primemodwalk3d` (`primes_modwalk3d`, `prime_modwalk3d`, `primesmodwalk3d`, `primeresiduewalk3d`), `ulamspiral` (`ulam`, `primespiral`), `sacksspiral` (`sacks`), `complexplot` (`domainplot`, `domaincoloring`), `densityplot` (`heatmap`, `contourf`), `plot3d` (`surfaceplot`, `fsurf`, `fmesh`, `ezsurf`), `parametricplot3d` (`spacecurve`, `fplot3`), `graphplot` (`drawgraph`), `matrixplot` (`arrayplot`, `imagesc`, `imshow`, `pcolor`, `image`), `save_png` (`savepng`), `save_jxl` (`savejxl`), `save` (`saveimage`, `export`).
- **Units**: `convert` (`unitconvert`), `siunits` (`tosi`, `baseunits`), `dimension` (`unitdimension`), `quantity`.
- **Session**: `assume` (`assumealso`), `forget`.
- **Operators by their names**: `times`, `rdivide`, `ldivide`, `power`, `plus`, `minus`, `uminus`, `uplus`, `mtimes`, `mpower`, `mldivide`, `mrdivide`, `colon`.
- **Arrays**: `linspace`, `logspace`, `meshgrid`, `ndgrid`, `repmat`, `repelem`, `horzcat`, `vertcat`, `cat`, `blkdiag`, `flip`, `fliplr`, `flipud`, `rot90`, `circshift`, `tril`, `triu`, `sortrows`, `issorted`, `setxor`, `ismember`, `isequal`, `numel`, `size`, `ndims`, `isempty`, `isrow`, `iscolumn`, `isvector`, `isscalar`, `ismatrix`, `isdiag`, `istril`, `istriu`, `bandwidth`, `isbanded`, `issymmetric`, `ishermitian`, `nnz`, `nonzeros`, `find`, `any`, `all`, `ind2sub`, `sub2ind`, `magic`, `invhilb`, `pascal`, `rosser`, `wilkinson`, `toeplitz`, `hankel`, `compan`, `hadamard`, `vander`, `ctranspose`, `nchoosek`, `perms`.
- **Sums and statistics of data**: `cumprod`, `cummax`, `cummin`, `bounds`, `peak2peak`, `maxk`, `mink`, `movmean`, `movsum`, `movmedian`, `movmax`, `movmin`, `movstd`, `movvar`, `movprod`, `movmad`, `trapz`, `cumtrapz`, `prctile` (`percentile`), `zscore`, `rescale` (`mat2gray`), `trimmean`, `mad`, `cov`, `corrcoef`, `histcounts`, `detrend`, `unwrap`, `accumarray`, `tabulate`, `tiedrank`.
- **Matrices taken apart**: `cond`, `rcond`, `vecnorm`, `orth`, `colspace`, `lu`, `chol`, `jordan`, `jordandecomposition`, `diagonalize`, `smithform`, `smithdecomposition`, `hermiteform`, `hermitedecomposition`, `schur`, `schurdecomposition`, `hess`, `hessenbergdecomposition`, `singularvaluedecomposition`, `sylvester`, `subspace`, `planerot`.
- **Polynomials as lists of coefficients, the highest first**: `poly`, `polyval`, `polyvalm`, `polyfit`, `polyder`, `polyint`, `conv`, `deconv`, `poly2sym`, `sym2poly`, `coeffs`, `horner`, `numden`, `quorem`, `interp1`, `spline`, `pchip`, `makima`, `interp2`, `gbasis` (`groebnerbasis`).
- **Signals**: `fft`, `ifft`, `fft2` (`fftn`), `ifft2` (`ifftn`), `fftshift`, `ifftshift`, `dct`, `idct`, `lfilter`, `conv2` (`convn`), `xcorr`, `upsample`, `downsample`.
- **Functions of numbers by the names of MATLAB**: `expm1`, `log1p`, `nthroot`, `realsqrt`, `reallog`, `realpow`, `nextpow2`, `pow2`, `idivide`, `sind`, `cosd`, `tand`, `cotd`, `secd`, `cscd`, `asind`, `acosd`, `atand`, `acotd`, `asecd`, `acscd`, `atan2d`, `sinpi`, `cospi`, `deg2rad`, `rad2deg`, `cart2pol`, `pol2cart`, `cart2sph`, `sph2cart`, `rat`, `bitand`, `bitor`, `bitxor`, `bitshift`, `bitcmp`, `bitget`, `bitset`, `erfcx`, `dawson`, `betaln`, `ssinint`, `airy`, `besselh`, `whittakerm`, `whittakerw`, `wrightomega`, `rectangularpulse`, `triangularpulse`, `jacobicd`, `jacobisd`, `jacobind`, `jacobidc`, `jacobinc`, `jacobisc`, `jacobins`, `jacobids`, `jacobics`, `jacobiam`, `jacobizeta`, `ellipticck`, `ellipticce`, `ellipticcpi`, `ellipke`, `ellipj`, `gammaincinv`, `betaincinv`.
- **Texts**: `num2str`, `int2str`, `mat2str`, `tostring` (`string`), `dec2bin`, `dec2hex`, `dec2base`, `bin2dec`, `hex2dec`, `base2dec`, `char`, `double`, `strcat`, `strlength` (`strlen`), `upper` (`toupper`), `lower` (`tolower`), `strtrim` (`strip`), `strrep`, `strfind`, `strsplit` (`split`), `strjoin`, `strcmp`, `strcmpi`, `strncmp`, `strncmpi`, `startswith`, `endswith`, `blanks`, `isletter`, `isspace`, `ischar` (`isstring`), `isnumeric`, `islogical`, `logical`, `sprintf`, `rats`.
- **The distribution functions of MATLAB**: `normpdf`, `normcdf`, `norminv`, `normrnd`, `normstat`, `unifpdf`, `unifcdf`, `unifinv`, `unifrnd`, `unifstat`, `exppdf`, `expcdf`, `expinv`, `exprnd`, `expstat`, `gampdf`, `gamcdf`, `gaminv`, `gamrnd`, `gamstat`, `betapdf`, `betacdf`, `betainv`, `betarnd`, `betastat`, `chi2pdf`, `chi2cdf`, `chi2inv`, `chi2rnd`, `chi2stat`, `tpdf`, `tcdf`, `tinv`, `trnd`, `tstat`, `fpdf`, `fcdf`, `frnd`, `fstat`, `lognpdf`, `logncdf`, `logninv`, `lognrnd`, `lognstat`, `wblpdf`, `wblcdf`, `wblinv`, `wblrnd`, `wblstat`, `raylpdf`, `raylcdf`, `raylinv`, `raylrnd`, `raylstat`, `binopdf`, `binocdf`, `binoinv`, `binornd`, `binostat`, `poisspdf`, `poisscdf`, `poissinv`, `poissrnd`, `poissstat`, `geopdf`, `geocdf`, `geoinv`, `geornd`, `geostat`, `nbinpdf`, `nbincdf`, `nbininv`, `nbinrnd`, `nbinstat`, `hygepdf`, `hygecdf`, `hygeinv`, `hygernd`, `hygestat`, `unidpdf`, `unidcdf`, `unidinv`, `unidrnd`, `unidstat`.
- **Random numbers**: `rng`, `rand` (`randomreal`), `randn`, `randi` (`randominteger`), `randperm`, `randsample`, `shuffle`, `random`.
- **Optimisation with exact numbers**: `linprog`, `intlinprog`, `lsqnonneg`.
- **Zeros, minima, integrals and differential equations of functions**: `fzero`, `fminbnd`, `fminsearch` (`fminunc`), `lsqcurvefit`, `quad` (`quadgk`, `quadl`, `quadv`), `integral2` (`quad2d`, `dblquad`), `integral3` (`triplequad`), `ode45` (`ode23`, `ode113`, `ode78`, `ode89`, `ode15s`, `ode23s`, `ode23t`, `ode23tb`), `fsolve`, `feval`, `bsxfun`, `funm`, `lsqminnorm`, `str2num` (`str2sym`), `str2double`, `finverse`, `compose`, `symvar`, `has`, `children`, `lhs`, `rhs`, `isalways`, `combine`, `rewrite`, `potential`, `vectorpotential`, `equationstomatrix`, `eliminate`, `isolate`, `assumptions`, `ttest`, `ttest2`, `ztest`, `vartest`, `vartest2`, `splitapply`, `assert`, `error`.
- **Data with gaps and outliers, groups**: `isoutlier`, `rmoutliers`, `ismissing`, `rmmissing`, `fillmissing`, `discretize`, `findgroups`, `ecdf`, `cplxpair`, `regress`.
- **More on graphs**: `bfsearch`, `dfsearch`, `successors`, `predecessors`, `nearest`, `allpaths`, `hascycles`, `isdag`, `transclosure`, `transreduction`, `biconncomp`, `cutvertices` (`articulationvertices`), `bridges`, `centrality`, `addedge`, `rmedge`, `addnode`, `rmnode`, `findedge`, `flipedge`.
- **Geometry of points**: `polyarea`, `inpolygon`, `convhull`, `convexhull`, `delaunay`, `voronoi`, `rectint`, `dsearchn`, `rotx`, `roty`, `rotz`.
- **Kinds of whole numbers, values of the largest size, the session**: `int8`, `int16`, `int32`, `int64`, `uint8`, `uint16`, `uint32`, `uint64`, `eigs`, `svds`, `disp`, `fprintf`, `readmatrix` (`csvread`, `dlmread`), `writematrix` (`csvwrite`, `dlmwrite`), `tic`, `toc`, `who`, `clear`.
- **Charts of data (they take what is chosen for a picture, and save_png / save_jxl save them)**: `lineplot` (`dataplot`, `comet`), `scatter`, `bar`, `barh`, `stairs`, `stem`, `errorbar`, `pie` (`pie3`), `loglog`, `semilogx`, `semilogy`, `contour` (`fcontour`), `quiver`, `plot3` (`comet3`), `scatter3`, `surf` (`mesh`, `surfl`), `spy`, `boxplot` (`boxchart`), `imread`, `imframes`, `imwrite`.
- **More of MATLAB: arrays of more sizes, matrices by the places of their entries, data with gaps**: `condest`, `normest`, `filter2`, `newline`, `meanabs`, `nansum`, `nanmean`, `nanmax`, `nanmin`, `nanstd`, `nanvar`, `nanmedian`, `single`, `cast`, `class`, `isa`, `sparse`, `full`, `issparse`, `nzmax`, `spones`, `speye`, `spdiags`, `squeeze`, `permute`, `ipermute`, `num2cell`, `cell2mat`, `signim`, `padecoef`, `crosstab`, `pdist`, `squareform`, `normfit`, `expfit`, `partialcorr`, `del2`, `interpft`, `datasample`, `quadprog`, `lsqlin`, `lsqnonlin`, `mellin`, `anova1`, `polyeig`, `pca`, `mkpp`, `unmkpp`, `ppval`, `polynomialreduce`, `griddata`, `tsearchn`, `erase`, `extractbefore`, `extractafter`, `extractbetween`, `insertafter`, `insertbefore`, `pad`, `isstrprop`.
- **More charts, and pictures as matrices**: `stem3`, `quiver3`, `contour3`, `waterfall`, `compass`, `feather`, `polarscatter`, `rgb2gray`, `im2double`, `im2uint8`, `im2uint16`, `imcomplement`, `imcrop`, `imresize`, `imrotate`, `imfilter`, `imhist`, `imbinarize`, `htrans`, `ihtrans`.
- **The calendar**: `datenum` (`datetime`), `datevec`, `datestr`, `weekday`, `eomday`, `calendar`, `addtodate`, `etime`, `juliandate`, `mjuliandate` (`modifiedjuliandate`), `posixtime`, `dayofyear`, `isweekend`, `leapyear` (`isleapyear`, `is_leap_year`), `yeardays`, `now`, `clock`, `date` (`today`), `cputime`.
- **Patterns of texts**: `regexp`, `regexpi`, `regexprep`, `regexptranslate`, `strjust`, `validatestring`.
- **Tests of ranks and counts, groups of points**: `ranksum`, `signrank`, `signtest`, `kruskalwallis`, `friedman`, `vartestn` (`bartlett`), `fishertest`, `chi2gof`, `kstest`, `kstest2`, `jbtest`, `anova2`, `mahal`, `mvnpdf`, `mvnrnd`, `ksdensity`, `jackknife`, `bootstrp`, `grpstats`, `linkage`, `cluster`, `kmeans`, `fitlm`.
- **Records and tables**: `struct`, `isstruct`, `fieldnames`, `numfields`, `getfield`, `setfield`, `rmfield`, `isfield`, `orderfields`, `struct2cell`, `cell2struct`, `structfun`, `dictionary`, `keys`, `values`, `entries`, `numentries`, `iskey`, `lookup`, `remove`, `iscell`, `iscellstr`, `cell`.
- **More special functions**: `meijerg`, `lerchphi` (`lerch`).
- **Expressions in an unknown function and its derivatives**: `functionalderivative` (`eulerlagrange`), `odetovectorfield`.
- **The least value under conditions**: `fmincon`.
- **Matrices with names, a surface, colours, counts of pairs**: `gallery`, `peaks`, `colormap`, `histcounts2`, `pcg` (`gmres`, `bicg`, `bicgstab`, `cgs`, `minres`, `qmr`, `symmlq`, `tfqmr`), `lsqr`.
- **Pairs of matrices, forms in another order**: `balance`, `qz`, `ordqz`, `gsvd`, `gsvddecomposition`, `ordschur`, `ordeig`, `rsf2csf`, `cdf2rdf`.
- **Runs and spreads of samples, the normal law of several variables, laws fitted to data**: `runstest`, `ansaribradley`, `lillietest`, `mvncdf`, `cmdscale`, `procrustes`, `canoncorr`, `nnmf`, `poissfit`, `unifit`, `raylfit`, `lognfit`, `binofit`, `gamfit`, `wblfit`, `betafit`, `nbinfit`, `mle`, `fitdist`.
- **Values at both ends, equations that look back, heat in one dimension, least values without slopes**: `bvp4c` (`bvp5c`), `bvpinit`, `deval`, `dde23`, `pdepe`, `patternsearch`, `particleswarm`, `simulannealbnd`, `ga`, `globalsearch` (`multistart`), `fminimax`, `fgoalattain`, `pdeval`.
- **Tables of named columns, tables with times, values of a few kinds**: `istable`, `timetable`, `istimetable`, `height`, `width`, `head`, `tail`, `varnames`, `takerows`, `takevars`, `addvars`, `removevars`, `renamevars`, `movevars`, `topkrows`, `table2array` (`table2cell`), `array2table` (`cell2table`), `struct2table`, `table2struct`, `timetable2table`, `table2timetable`, `innerjoin`, `outerjoin`, `varfun`, `rowfun`, `groupsummary`, `groupcounts`, `summary`, `stack`, `unstack`, `readtable`, `writetable`, `isregular`, `lag`, `retime`, `synchronize`, `categorical`, `iscategorical`, `categories`, `countcats`, `isundefined`, `isordinal`, `iscategory`, `addcats`, `removecats`, `renamecats`, `reordercats`, `mergecats`, `setcats`, `grp2idx`, `cellstr`.
- **More charts of data**: `bar3`, `histogram2`, `surfc` (`meshc`), `streamline`, `polarhistogram` (`rose`), `dendrogram`, `bubblechart`, `fimplicit3`, `isosurface`, `triplot`, `trisurf` (`trimesh`), `pareto`, `fill` (`patch`), `ribbon`, `normplot` (`qqplot`, `probplot`), `parallelplot`, `swarmchart`, `violinplot`, `histfit`, `gscatter`, `andrewsplot`, `bar3h`, `cdfplot`, `plotmatrix`, `stackedplot`, `biplot`, `silhouette`.
- **Expressions as code, their shared parts, and as a picture of text**: `ccode`, `fortran`, `matlabcode` (`octavecode`), `matlabfunction`, `subexpr`, `cse`, `pretty`.
- **Systems of control: one input, one output, as a ratio of polynomials in s (running time) or z (sampled)**: `tf`, `zpk`, `ss`, `tfdata`, `zpkdata`, `ssdata`, `tf2sym`, `tf2ss`, `ss2tf`, `tf2zp` (`tf2zpk`), `zp2tf`, `ss2zp`, `zp2ss`, `isct`, `isdt`, `isproper`, `isstable`, `pole`, `zero` (`tzero`), `dcgain`, `damp`, `parallel`, `feedback`, `minreal`, `evalfr`, `freqresp`, `bode` (`nichols`), `nyquist`, `margin`, `allmargin`, `hinfnorm` (`getpeakgain`), `h2norm`, `step`, `impulse`, `lsim`, `stepinfo`, `c2d`, `d2c`, `pid`, `rlocus`, `esort`, `dsort`, `ctrb`, `obsv`, `lyap`, `dlyap`, `gram`, `care` (`icare`), `dare` (`idare`), `lqr`, `dlqr`, `lqe`, `acker`, `place`, `hsvd`, `balreal`, `balred`, `covar`, `dlqe`, `lyapchol`, `dlyapchol`, `initial`, `gensig`, `lqi`, `ss2ss`, `ord2`, `d2d`, `reg`, `filt`, `bodeplot`, `nyquistplot`, `pzplot` (`pzmap`), `rlocusplot`, `stepplot`, `impulseplot`, `initialplot`, `nicholsplot`.
- **Filters and spectra of signals: windows, filters of Butterworth, Chebyshev, Cauer and Bessel, filters by windows, responses, spectra, peaks, prediction**: `rectwin` (`boxcar`), `hann`, `hanning`, `hamming`, `blackman`, `blackmanharris`, `nuttallwin`, `flattopwin`, `triang`, `bohmanwin`, `parzenwin`, `gausswin`, `tukeywin`, `kaiser`, `chebwin`, `barthannwin`, `buttap`, `cheb1ap`, `cheb2ap`, `ellipap`, `besselap`, `butter`, `cheby1`, `cheby2`, `ellip`, `besself`, `buttord`, `cheb1ord`, `cheb2ord`, `ellipord`, `lp2lp`, `lp2hp`, `lp2bp`, `lp2bs`, `bilinear`, `iirnotch`, `iirpeak`, `fir1`, `kaiserord`, `sgolay`, `sgolayfilt`, `freqz`, `freqs`, `grpdelay`, `impz`, `stepz`, `filtfilt`, `sosfilt`, `sos2tf`, `tf2sos`, `zp2sos`, `isminphase`, `ismaxphase`, `islinphase`, `firtype`, `polystab`, `periodogram`, `pwelch`, `cpsd`, `mscohere`, `tfestimate`, `spectrogram`, `bandpower`, `rssq`, `peak2rms`, `enbw`, `mag2db` (`db`), `db2mag`, `pow2db`, `db2pow`, `rceps`, `goertzel`, `czt`, `dftmtx`, `cconv`, `envelope`, `findpeaks`, `medfilt1`, `upfirdn`, `decimate`, `buffer`, `xcov`, `levinson`, `aryule`, `arburg`, `lpc`, `poly2rc`, `rc2poly`, `sawtooth`, `rectpuls`, `tripuls`, `gauspuls`, `chirp`, `diric`, `zplane`.
- **More of the design of filters: least squares, samples of the response, equal ripples, another rate, lattices, phases, delays**: `firls`, `firpm` (`remez`), `fir2`, `resample`, `phasez`, `phasedelay`, `zerophase`, `finddelay`, `dtw`, `residuez`, `tf2latc`, `latc2tf`, `latcfilt`, `fftfilt`, `filtic`.
- **A function of a complex number as a shape of four dimensions: frames are the third, colours the fourth**: `plot4d`, `view4d` (`plot4dview`, `look4d`, `snapshot4d`).
- **Quick looks: the commands that save, with small pictures that are packed fast**: `save_png_fast` (`savepngfast`), `save_jxl_fast` (`savejxlfast`), `save_fast` (`savefast`).
- **The earth: ways on a sphere and on an ellipsoid. Angles are degrees unless "radians" is said last; an ellipsoid is [half axis, eccentricity], the radius of a sphere or a name like "wgs84" (metres); without one, lengths are arcs of the sphere in degrees**: `azimuth`, `reckon`, `haversine`, `geoddistance`, `geodreckon`, `geodarea`, `areaint`, `areaquad`, `departure`, `antipode`, `meanm`, `track2`, `track1`, `gcwaypts`, `scircle1`, `scircle2`, `legs`, `gc2sc`, `gcxgc`, `scxsc`, `gcxsc`, `rhxrh`, `interpm`.
- **The earth: places in space. geodetic = latitude, longitude and height above the ellipsoid (WGS84 in metres unless given); ecef = x, y, z from the middle of the earth; enu = east, north, up and ned = north, east, down at a place; aer = course, elevation and distance as seen from a place**: `geodetic2ecef`, `ecef2geodetic`, `lla2ecef`, `ecef2lla`, `geodetic2enu`, `enu2geodetic`, `geodetic2ned`, `ned2geodetic`, `geodetic2aer`, `aer2geodetic`, `ecef2enu`, `enu2ecef`, `ecef2ned`, `ned2ecef`, `ecef2aer`, `aer2ecef`, `enu2aer`, `aer2enu`, `ned2aer`, `aer2ned`, `ecef2enuv`, `enu2ecefv`, `ecef2nedv`, `ned2ecefv`, `ecefoffset`, `elevation`.
- **The earth: ellipsoids, their radii and latitudes of other kinds**: `referenceellipsoid`, `wgs84ellipsoid`, `referencesphere`, `meridianarc`, `meridianfwd`, `spheroid`, `axes2ecc`, `majaxis`, `minaxis`, `ecc2flat`, `flat2ecc`, `ecc2n`, `n2ecc`, `rcurve`, `rsphere`, `convertlat`, `geocentriclatitude`, `parametriclatitude`, `geodeticlatitudefromgeocentric`, `geodeticlatitudefromparametric`.
- **The earth: units of lengths and angles (exact), angles as texts, zones**: `km2nm`, `km2sm`, `nm2km`, `nm2sm`, `sm2km`, `sm2nm`, `deg2km`, `km2deg`, `deg2nm`, `nm2deg`, `deg2sm`, `sm2deg`, `rad2km`, `km2rad`, `rad2nm`, `nm2rad`, `rad2sm`, `sm2rad`, `unitsratio`, `earthradius`, `distdim`, `angledim`, `todegrees`, `toradians`, `fromdegrees`, `fromradians`, `wrapto180` (`npi2pi`), `wrapto360` (`zero22pi`), `wraptopi`, `wrapto2pi`, `roundn`, `dms2degrees`, `dm2degrees`, `degrees2dms`, `degrees2dm`, `angl2str`, `str2angle`, `timezone`, `utmzone`.
- **The earth: maps. projfwd("map", lat, lon, choices) gives [x, y], projinv("map", x, y, choices) gives [lat, lon]**: `projfwd` (`mfwdtran`), `projinv` (`minvtran`), `deg2utm`, `utm2deg`.
- **Networks of neurons: the functions of their units (exact; of a name a formula that has a slope), for a number or every entry of a list**: `relu` (`poslin`), `leakyrelu`, `clippedrelu`, `elu`, `gelu`, `swish` (`silu`), `mish`, `softplus`, `softsign` (`elliotsig`), `sigmoid` (`logsig`, `logistic`), `tansig`, `purelin`, `satlin`, `satlins`, `hardlim`, `hardlims`, `radbas`, `tribas`, `netinv`, `softmax`, `compet`.
- **Networks of neurons: losses (exact; rows are observations, a flat list holds one number of every observation)**: `mse`, `sse`, `mae`, `l2loss`, `l1loss`, `huber`, `crossentropy`.
- **Networks of neurons: the operations of layers**: `fullyconnect`, `dlconv`, `maxpool`, `avgpool`, `batchnorm`, `layernorm`, `embed`, `lstm`, `gru`, `onehotencode`, `onehotdecode`, `ind2vec`, `vec2ind`, `confusionmat`, `mapminmax`, `mapstd`, `divideblock`, `dlgradient`, `sgdmupdate`, `adamupdate`, `rmspropupdate`.
- **Networks of neurons: a network is a list of layers [weights, biases, "function"], a row of the weights for every unit**: `feedforwardnet` (`fitnet`), `patternnet`, `predict` (`forward`), `classify`, `perform`, `trainnet` (`train`), `perceptron`, `newlind`, `newrbe`.

Constants:

| Name | Also | Meaning |
| --- | --- | --- |
| `pi` | `π` | circle constant |
| `e` | `ℯ`, `ⅇ` | Euler's number |
| `i` | `j`, `ⅈ`, `ⅉ` | imaginary unit |
| `inf` | `infinity`, `oo`, `∞` | positive infinity |
| `ComplexInfinity` | `zoo`, `cinf` | complex infinity, 1/0 |
| `undefined` | `nan`, `indeterminate` | no value, 0/0 |
| `true` |  | truth value |
| `false` |  | truth value |
| `tau` | `τ` | 2 pi |
| `halfpi` |  | pi/2 |
| `deg` | `degree`, `°` | one degree in radians, pi/180 |
| `sqrtpi` | `√π` | square root of pi |
| `eta` | `towermax`, `η` | e^(1/e): the largest base whose infinite tower converges |
| `towermin` |  | e^(-e): the smallest base whose infinite tower converges |
| `einv` |  | 1/e |
| `gelfond` |  | Gelfond's constant e^pi |
| `gelfondschneider` |  | Gelfond-Schneider constant 2^sqrt(2) |
| `ramanujan` |  | Ramanujan's constant e^(pi sqrt 163) |
| `phi` | `golden`, `goldenratio`, `φ`, `ϕ` | golden ratio (1 + sqrt 5)/2 |
| `silver` |  | silver ratio 1 + sqrt 2 |
| `plastic` |  | plastic number: x^3 = x + 1 |
| `supergolden` |  | supergolden ratio: x^3 = x^2 + 1 |
| `tribonacci` |  | tribonacci constant: x^3 = x^2 + x + 1 |
| `sqrt2` | `√2` | square root of 2 |
| `sqrt3` | `√3` | square root of 3 |
| `sqrt5` | `√5` | square root of 5 |
| `ln2` |  | natural logarithm of 2 |
| `ln10` |  | natural logarithm of 10 |
| `eulergamma` | `gamma`, `γ` | Euler-Mascheroni constant (gamma(x) with brackets is the function) |
| `catalan` |  | Catalan's constant |
| `apery` | `zeta3` | Apery's constant zeta(3) |
| `zeta2` | `basel` | zeta(2) = pi^2/6 |
| `omega` | `Ω`, `ω` | omega constant W(1): omega e^omega = 1 |
| `dottie` |  | Dottie number: cos x = x |
| `lemniscate` | `ϖ` | lemniscate constant Gamma(1/4)^2/(2 sqrt(2 pi)) |
| `gauss` |  | Gauss's constant 1/agm(1, sqrt 2) |
| `parabolic` |  | universal parabolic constant sqrt 2 + ln(1 + sqrt 2) |
| `levy` |  | Levy's constant e^(pi^2/(12 ln 2)) |
| `khinchin` |  | Khinchin's constant |
| `gompertz` |  | Euler-Gompertz constant |
| `soldner` |  | Ramanujan-Soldner constant: zero of the logarithmic integral |
| `laplace` | `laplacelimit` | Laplace limit |
| `erdosborwein` |  | Erdos-Borwein constant: sum of 1/(2^n - 1) |
| `cahen` |  | Cahen's constant |
| `recfib` |  | reciprocal Fibonacci constant: sum of 1/F_n |
| `niven` |  | Niven's constant |
| `mertens` |  | Meissel-Mertens constant |
| `twinprime` |  | twin prime constant |
| `flintmax` |  | 2^53: up to it every whole number is a double of MATLAB |
| `realmax` |  | the largest double of MATLAB |
| `realmin` |  | the least normal positive double of MATLAB |
| `intmax` |  | the largest 32-bit whole number |
| `intmin` |  | the least 32-bit whole number |

`gamma` and `γ` without brackets are Euler's constant, `gamma(x)` and `Γ(x)` are the function.
The constants with a closed form are replaced by it (`phi` is `1/2 + sqrt(5)/2`); the others
are kept as names and have numeric values of any precision.

## Notes

- Functions are principal values: `(-8)^(1/3)` is `1 + 1.732...i`, `sqrt(x^2)` is not `x`
  without an assumption; with `-r` the real cube root `-2` is taken.
- `0.(3)` is the repeating decimal 0.333... = 1/3, and `0.1(6)` is 1/6.
- A result that is only numeric is printed as a decimal number; solutions found by a numeric
  search are printed with `~` and a note that there may be others.
- Sums, integrals and limits that are not found are reported as errors or given back
  unevaluated; nothing is guessed.
- `0/0`, `0^0` and `inf - inf` are `undefined`; `1/0` is `ComplexInfinity`, and
  `1/ComplexInfinity` is 0: `cot(x)` is `1/tan(x)` and vanishes where `tan(x)` has its poles.
- An equation that holds on whole stretches of numbers (`abs(x)/x = 1`, `floor(x) = 2`) gets a
  few examples and a note; over the real numbers an inequality describes such a set.

## Development and license

`make test` runs the unit tests (number theory, iterates) and the integration tests, which
run the program on fixed inputs: `tests/cli.rs` (with a stand-in `tet`, and with the real one
when it is installed; a drawing asks `tet` once for every place, many runs at a time), `tests/iterates.rs` (fractional and complex iterates: closed forms,
values checked with other software, the law `f[s](f[t](x)) = f[s + t](x)`), `tests/round3.rs`
(lists, statistics, distributions, transforms, special functions and more), `tests/round4.rs`
(equations in one unknown and with quaternion unknowns, units, graphs, plots),
`tests/round5.rs` (numbers in a base, walks, pictures of characters, saved pictures: PNG
files byte for byte, JPEG XL files decoded and compared with them), `tests/help.rs` (every
example of `--help` runs, every function and constant has one, and every picture of `jxls/`
stands there with its input, which runs with small sizes) and `tests/coverage.rs`
with the tables of `tests/coverage/`: more than 52000 inputs with their answers, among them
every name of every function, too few and too many arguments, numbers against exact
arithmetic, functions against values of 25 digits from mpmath, identities of matrices, lists
and polynomials, derivatives of integrals, texts, programs, refusals, what MATLAB has area by
area, the shapes of four dimensions through every value of every choice, and the inputs
whose answers agreed with other software (the tables `checked_by_*`). A test fails when a
function has no row.
What MATLAB has was compared with GNU Octave (more than 1200 cases), SciPy and NumPy (more
than 1800 checks), python-control, statsmodels and scikit-learn, geographiclib, PROJ and
pymap3d (more than 1200 cases) and mpmath while it was written; the scripts are not part of
the repository.
`make lint` checks the formatting and runs clippy with warnings as errors.

Licensed under the GNU Affero General Public License, version 3 only (AGPL-3.0-only); see
[LICENSE](LICENSE).
