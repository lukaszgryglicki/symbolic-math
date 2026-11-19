# symbolic-math

A computer algebra system for the command line, written in Rust. It simplifies expressions,
solves equations, inequalities and systems of them, differentiates, integrates, takes limits,
sums series, solves differential, functional and Diophantine equations and recurrences, works
with matrices, tensors, lists, whole numbers, special functions, integral transforms,
statistics, geometry, graphs and quantities with units, applies a function a fractional or
complex number of times, draws plots with characters, and evaluates everything to as many
digits as asked for.

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

Rust (edition 2024, tested with 1.97 and 1.98), a C compiler and `m4` are needed: the only
dependency, [rug](https://crates.io/crates/rug), builds GMP, MPFR and MPC from source. FreeBSD,
Linux and macOS are supported.

```sh
cargo build --release --locked
./target/release/symbolic-math 'integrate(x*e^x, x)'
```

The Makefile builds the same executable and installs it:

```sh
make release    # target/release/symbolic-math
make static     # a static executable (on macOS: one that needs the system libraries only)
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
printed, with a note.

## Syntax

- **Numbers**: `42`, `3.14`, `1e-3`, `0x1F`, `0b101`, `0o17`, `2+3i`, `¾`. Decimals are exact
  until a numeric value is needed.
- **Operators**: `+ - * /`, `^` (right to left: `2^3^2` is `2^9`), `^^` (binds tighter than
  `^`), `!` and `!!` (factorials), `%` or `mod` (remainder), `'` (derivative: `f'`, `f''`), `°`
  (degrees), `|x|` (absolute value), `±` or `+/-` (both values: `x = 3 +/- 1`).
- **Multiplication** may be left out: `2x`, `2 pi r`, `(x+1)(x-1)`, `sin(x)cos(x)`. A single
  letter in front of a bracket multiplies (`x(x+1)`), a longer name must be a function.
- **Relations**: `=`, `!=`, `<`, `<=`, `>`, `>=`; `x -> a` in `limit`, `0+` and `0-` for one side.
  `and`, `or`, `not` (`&&`, `∧`, `∨`, `¬`) join them.
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
- **Complex numbers**: `abs` (`mag`, `polar_r`, `radius`), `mod`, `norm` (`‖`), `abs2` (`mag2`, `norm_sqr`, `normsq`), `l1_norm` (`l1`, `manhattan`, `taxicab`), `arg` (`phase`, `polar_theta`, `theta`), `to_polar`, `re` (`real`, `ℜ`), `im` (`imag`, `ℑ`), `conj` (`conjugate`), `inv` (`recip`, `inverse`, `finv`), `sign` (`sgn`, `signum`), `cis`, `scale`, `unscale`, `fdiv`, `complex` (`new`), `rect` (`rectform`), `polar` (`from_polar`, `polarform`), `floor` (`⌊`), `ceil` (`ceiling`, `⌈`), `round`, `trunc`, `frac` (`fract`), `minabs`, `maxabs`, `avg` (`mean`, `average`), `min`, `max`, `hypot`, `agm`, `is_nan` (`isnan`), `is_infinite` (`isinf`, `isinfinite`), `is_finite` (`isfinite`), `is_normal` (`isnormal`).
- **Special functions**: `gamma` (`tgamma`, `Γ`), `lgamma` (`loggamma`, `lngamma`), `factorial` (`fact`), `factorial2` (`doublefactorial`), `binomial` (`choose`), `beta`, `digamma` (`psi`, `ψ`), `polygamma`, `harmonic`, `zeta` (`hurwitzzeta`, `ζ`), `erf`, `erfc`, `erfi`, `ei`, `li`, `li2` (`dilog`), `si`, `ci`, `shi`, `chi`, `lambertw` (`w`, `productlog`), `uppergamma` (`gammainc`, `incgamma`), `lowergamma`, `gammaq` (`gammaregularized`), `gammap`, `expint` (`expintegrale`), `betainc`, `betaregularized`, `erfinv` (`inverseerf`), `erfcinv` (`inverseerfc`), `fresnels`, `fresnelc`, `sinc`, `besselj`, `bessely`, `besseli`, `besselk`, `hankel1` (`hankelh1`), `hankel2` (`hankelh2`), `sphericalbesselj`, `sphericalbessely`, `airyai`, `airybi`, `airyaiprime`, `airybiprime`, `struveh`, `struvel`, `hyp0f1` (`hypergeometric0f1`), `hyp1f1` (`hypergeometric1f1`, `kummerm`), `hyperu` (`hypergeometricu`, `kummeru`), `hyp2f1` (`hypergeometric2f1`), `hypergeometric` (`hypergeometricpfq`, `pfq`), `polylog`, `dirichleteta`, `elliptick` (`ellipk`), `elliptice` (`ellipe`), `ellipticf` (`ellipf`), `ellipticpi` (`ellippi`), `ellipticnome` (`nome`), `jacobisn`, `jacobicn`, `jacobidn`, `elliptictheta` (`jtheta`), `legendrep`, `legendreq`, `chebyshevt`, `chebyshevu`, `hermiteh` (`hermite`), `laguerrel` (`laguerre`), `gegenbauerc` (`gegenbauer`), `jacobip`, `sphericalharmonicy` (`sphericalharmonic`, `ylm`), `heaviside` (`heavisidetheta`), `unitstep`, `diracdelta` (`dirac`).
- **Tetration**: `tet`, `slog` (`ate`), `ssrt` (`sroot`).
- **Algebra**: `simplify`, `expand`, `factor`, `collect`, `together`, `cancel`, `apart`, `numer` (`numerator`), `denom` (`denominator`), `subst` (`substitute`, `replace`), `coeff` (`coefficient`), `degree`, `quo` (`quotient`), `rem` (`remainder`), `gcd`, `lcm`, `resultant`, `discriminant`, `roots`, `trigexpand` (`expand_trig`), `trigreduce`, `rewrite_exp` (`toexp`), `N` (`evalf`, `approx`, `numeric`), `rationalize`, `identify`.
- **Solving**: `solve`, `qsolve` (`quatsolve`), `nsolve`, `dsolve`, `rsolve`.
- **Calculus**: `diff` (`derivative`, `∂`), `D`, `integrate` (`int`, `integral`, `∫`), `nintegrate` (`nint`).
- **Areas volumes and regions**: `area`, `volume`, `perimeter` (`circumference`), `surfacearea`, `centroid`, `arclength`, `revolutionvolume`, `revolutionarea`, `rectangle`, `disk` (`disc`), `annulus`, `sector`, `ellipse`, `triangle`, `polygon`, `box` (`cuboid`), `ball`, `shell`, `ellipsoid`, `simplex`, `tetrahedron`, `cylinder`, `cone`.
- **Limits series and sums**: `limit` (`lim`), `series` (`taylor`), `sum` (`summation`, `∑`), `product` (`prod`, `∏`), `grad` (`gradient`, `∇`), `divergence`, `curl`, `laplacian`, `jacobian`, `hessian`, `minimize`, `maximize`.
- **Linear algebra**: `det` (`determinant`), `transpose`, `trace` (`tr`), `rank`, `rref`, `nullspace` (`kernel`), `eigenvalues` (`eigvals`), `eigenvectors` (`eigvecs`), `charpoly`, `dot`, `cross`, `identity` (`eye`), `linsolve`.
- **Complex parts functional equations and iterates**: `complexexpand`, `funsolve` (`functionalsolve`), `iterate` (`fiterate`).
- **Tensors**: `dimensions` (`dims`, `shape`), `tensorrank` (`arraydepth`), `flatten`, `reshape`, `outer` (`tensorproduct`), `contract` (`tensorcontract`), `inner`, `kron` (`kroneckerproduct`), `levicivita`, `signature`, `kroneckerdelta`, `symmetrize`, `antisymmetrize`, `wedge`, `hodge` (`hodgestar`), `christoffel`, `riemann`, `ricci`, `ricciscalar`, `einstein`, `kretschmann`, `geodesics`, `covariant`, `raiseindex`, `lowerindex`.
- **Geometry**: `line`, `plane`, `circle`, `sphere`, `distance`, `angle`, `projection`, `intersection`, `equation`, `midpoint`, `normalize` (`unit`), `triple`, `collinear`, `coplanar`, `circumcircle` (`circumsphere`), `incircle`, `solidangle`, `ballvolume`, `spherearea`, `rotationmatrix`, `rotationquat`, `rotate`, `reflect`, `topolar` (`tospherical`), `frompolar` (`fromspherical`), `regularpolygon`, `polytope`.
- **Functions of matrices and decompositions**: `adjugate` (`adj`), `matrixexp` (`expm`), `matrixpower`, `matrixsqrt` (`sqrtm`), `matrixlog` (`logm`), `matrixfunction`, `gramschmidt` (`orthogonalize`), `cholesky`, `qr`, `pinv` (`pseudoinverse`), `singularvalues`, `diag`, `zeros`, `ones`, `hilbert`, `vandermonde`.
- **Quaternions and domains**: `quat` (`quaternion`), `domain`.
- **Number theory**: `isprime` (`is_prime`, `primeq`), `iscomposite` (`compositeq`), `isprimepower` (`primepowerq`), `isperfect` (`perfectnumberq`), `iscoprime` (`coprime`, `coprimeq`), `primes`, `primefactors`, `ifactors` (`factorinteger`), `primorial`, `carmichael` (`carmichaellambda`), `liouville` (`liouvillelambda`), `mangoldt` (`mangoldtlambda`), `legendre` (`legendresymbol`), `farey` (`fareysequence`), `squaresr`, `partitionsq`, `eulerian`, `superfactorial`, `hyperfactorial`, `pell`, `egyptian`, `primitiveroots` (`primitiverootlist`), `sqrtmodall` (`sqrtmods`), `pochhammer` (`rising`, `risingfactorial`), `fallingfactorial` (`falling`, `factorialpower`), `nextprime`, `prevprime`, `prime` (`nthprime`), `primepi`, `divisors`, `totient` (`eulerphi`), `sigma` (`divisorsigma`), `mobius` (`moebius`), `fibonacci` (`fib`), `lucas`, `bernoulli`, `eulernumber`, `catalannumber`, `powmod`, `modinv`, `jacobi`, `isqrt`, `cfrac`, `base`, `frombase`, `digitsum`, `issquare`, `issquarefree`, `perfectpower`, `primenu`, `primeomega`, `radical`, `kronecker`, `sqrtmod`, `ord` (`multiplicativeorder`), `primitiveroot`, `discretelog` (`dlog`), `crt`, `egcd` (`gcdex`), `iroot`, `sumoftwosquares`, `bell`, `stirling1`, `stirling2`, `partitions`, `derangements` (`subfactorial`), `multinomial`, `convergents`.
- **Lists**: `range`, `table`, `length`, `first`, `last`, `rest`, `most`, `part`, `take`, `drop`, `reverse`, `sort`, `sortby`, `rotateleft`, `rotateright`, `append`, `prepend`, `join` (`concat`), `insert`, `delete`, `union`, `complement` (`setminus`), `unique` (`deleteduplicates`), `contains` (`memberq`), `count`, `position`, `total`, `accumulate` (`cumsum`), `differences`, `ratios`, `tally`, `zip`, `permutations`, `subsets`, `tuples`, `cartesianproduct`, `partition` (`chunks`), `map`, `select` (`filter`), `reject`, `countif`, `anytrue`, `alltrue`, `findfirst`, `fold` (`reduce`), `foldlist`, `nestlist`, `nest`, `fixedpoint`, `apply`, `array`.
- **Programs and logic**: `if`, `piecewise`, `while`, `do`, `for`, `block`, `and`, `or`, `not`, `xor`, `implies`, `islist` (`listq`), `isnumber` (`numberq`), `isinteger` (`integerq`), `isrational`, `iseven` (`evenq`), `isodd` (`oddq`), `isreal`, `ispositive`, `isnegative`, `isnonnegative`.
- **Statistics**: `median`, `mode` (`commonest`), `variance`, `stddev` (`standarddeviation`), `pvariance`, `pstddev`, `covariance`, `correlation`, `quantile` (`inversecdf`), `quartiles`, `iqr` (`interquartilerange`), `moment`, `centralmoment`, `skewness`, `kurtosis`, `geometricmean`, `harmonicmean`, `rms` (`rootmeansquare`), `meandeviation`.
- **Probability**: `pdf` (`pmf`, `probabilitydensity`), `cdf`, `mgf` (`momentgeneratingfunction`), `expectation` (`expectedvalue`), `probability`, `normaldist` (`normaldistribution`), `uniformdist` (`uniformdistribution`), `exponentialdist` (`exponentialdistribution`), `gammadist` (`gammadistribution`), `chisquaredist` (`chisquaredistribution`), `studenttdist` (`studenttdistribution`), `betadist` (`betadistribution`), `cauchydist` (`cauchydistribution`), `lognormaldist` (`lognormaldistribution`), `weibulldist` (`weibulldistribution`), `laplacedist` (`laplacedistribution`), `rayleighdist` (`rayleighdistribution`), `logisticdist` (`logisticdistribution`), `paretodist` (`paretodistribution`), `fdist` (`fratiodistribution`), `binomialdist` (`binomialdistribution`), `poissondist` (`poissondistribution`), `geometricdist` (`geometricdistribution`), `bernoullidist` (`bernoullidistribution`), `discreteuniformdist` (`discreteuniformdistribution`), `negativebinomialdist` (`negativebinomialdistribution`), `hypergeometricdist` (`hypergeometricdistribution`).
- **Numeric solutions lattices fits and equations in whole numbers**: `ndsolve`, `latticereduce` (`lll`), `findrelation` (`findintegerrelation`, `pslq`), `minpoly` (`minimalpolynomial`), `interpolate` (`interpolatingpolynomial`), `fit`, `leastsquares`, `criticalpoints` (`stationarypoints`), `modsolve` (`solvemod`), `diophantine`.
- **Transforms residues and approximants**: `laplace` (`laplacetransform`), `invlaplace` (`inverselaplace`, `ilaplace`, `inverselaplacetransform`), `fourier` (`fouriertransform`), `invfourier` (`inversefourier`, `ifourier`, `inversefouriertransform`), `fourierseries`, `ztransform`, `invztransform` (`inverseztransform`, `iztransform`), `residue`, `pade` (`padeapproximant`), `seriescoeff` (`seriescoefficient`).
- **Graphs**: `vertexlist` (`vertices`), `edgelist`, `vertexcount`, `edgecount`, `adjacencymatrix`, `weightmatrix` (`weightedadjacencymatrix`), `incidencematrix`, `laplacianmatrix` (`kirchhoffmatrix`), `adjacencygraph`, `degrees` (`degreesequence`, `vertexdegrees`), `indegrees`, `outdegrees`, `vertexdegree`, `neighbors` (`neighbours`, `adjacentvertices`), `components` (`connectedcomponents`), `strongcomponents` (`stronglyconnectedcomponents`), `isconnected` (`connectedq`), `shortestpath` (`findshortestpath`), `graphdistance`, `distancematrix` (`graphdistancematrix`), `eccentricity` (`vertexeccentricity`), `diameter` (`graphdiameter`), `graphradius`, `graphcenter` (`graphcentre`), `girth`, `spanningtree` (`minimumspanningtree`, `mst`), `spanningtreecount` (`treecount`), `isbipartite` (`bipartiteq`), `bipartition`, `istree` (`treeq`), `isacyclic` (`acyclicq`), `findcycle`, `topologicalsort` (`toposort`), `chromaticpolynomial`, `chromaticnumber`, `graphcoloring` (`vertexcoloring`), `iseulerian` (`eulerianq`), `eulerianpath` (`euleriancycle`, `eulertour`), `ishamiltonian` (`hamiltonianq`), `hamiltoniancycle` (`findhamiltoniancycle`), `hamiltonianpath`, `maxflow` (`maximumflow`), `maxmatching` (`maximummatching`), `maxclique` (`findclique`), `cliquenumber`, `maxindependentset` (`independentset`), `independencenumber`, `vertexcover` (`minvertexcover`), `complementgraph` (`graphcomplement`), `linegraph`, `subgraph` (`inducedsubgraph`), `isisomorphic` (`isomorphicq`, `graphisomorphic`), `pagerank`, `completegraph`, `cyclegraph`, `pathgraph`, `stargraph`, `wheelgraph`, `gridgraph`, `hypercubegraph`, `petersengraph`, `completebipartite` (`completebipartitegraph`).
- **Plots**: `plot`, `parametricplot`, `polarplot`, `listplot` (`pointplot`, `scatterplot`), `histogram` (`hist`), `implicitplot` (`contourplot`).
- **Units**: `convert` (`unitconvert`), `siunits` (`tosi`, `baseunits`), `dimension` (`unitdimension`), `quantity`.
- **Session**: `assume`, `forget`.

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
when it is installed), `tests/iterates.rs` (fractional and complex iterates: closed forms,
values checked with other software, the law `f[s](f[t](x)) = f[s + t](x)`), `tests/round3.rs`
(lists, statistics, distributions, transforms, special functions and more), `tests/round4.rs`
(equations in one unknown and with quaternion unknowns, units, graphs, plots) and
`tests/help.rs` (every example of `--help` runs, and every function and constant has one).
`make lint` checks the formatting and runs clippy with warnings as errors.

Licensed under the GNU Affero General Public License, version 3 only (AGPL-3.0-only); see
[LICENSE](LICENSE).
