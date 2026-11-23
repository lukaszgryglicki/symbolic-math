//! The equations with quaternion unknowns, the units, the graphs and the plots, with the
//! corrections that came with them: runs the program on fixed inputs and compares what it
//! prints. The solutions, the units, the graph commands and the dots of the plots were
//! compared with other software and with independent calculations first.

use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// What the program prints for an input, with `|` for line ends.
fn answer(options: &[&str], input: &str) -> String {
    let mut args = options.to_vec();
    args.push(input);
    let out = program()
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .expect("symbolic-math runs");
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim_end().replace('\n', "|");
    [text(&out.stdout), text(&out.stderr)]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("|")
}

fn check_with(options: &[&str], cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (input, want) in cases {
        let got = answer(options, input);
        if got != *want {
            failures.push(format!("{input}\n  want: {want}\n  got:  {got}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

fn check(cases: &[(&str, &str)]) {
    check_with(&[], cases);
}

const QUATERNION_EQUATIONS: &[(&str, &str)] = &[
    (r"i*x = j", r"x = -k"),
    (r"x*i = j", r"x = k"),
    (r"(1+j)*x = k", r"x = -i/2 + k/2|  ~ -0.5i + 0.5k"),
    (
        r"x*(2 - k) = 1 + i",
        r"x = 2/5 + 2i/5 - j/5 + k/5|  ~ 0.4 + 0.4i - 0.2j + 0.2k",
    ),
    (
        r"quat(1,2,3,4)*x = quat(0,1,0,1)",
        r"x = 1/5 - i/15 - j/15 + 2k/15|  ~ 0.2 - 0.066666666666666666666666666666667i - 0.066666666666666666666666666666667j + 0.13333333333333333333333333333333k",
    ),
    (
        r"x*quat(1,2,3,4) = quat(0,1,0,1)",
        r"x = 1/5 + 2i/15 + j/15 - k/15|  ~ 0.2 + 0.13333333333333333333333333333333i + 0.066666666666666666666666666666667j - 0.066666666666666666666666666666667k",
    ),
    (r"i*x + x*j = 1", r"no solution"),
    (
        r"i*x - x*j = 1 + k",
        r"x = -x3 + (x2 - 1)*i + x2*j + x3*k|x2 and x3 are any real numbers",
    ),
    (r"i*x*j + x = k", r"no solution"),
    (
        r"2x + i*x*j = 1 + j",
        r"x = 2/3 + i/3 + 2j/3 - k/3|  ~ 0.66666666666666666666666666666667 + 0.33333333333333333333333333333333i + 0.66666666666666666666666666666667j - 0.33333333333333333333333333333333k",
    ),
    (
        r"x*i - i*x = j",
        r"x = x0 + x1*i + k/2|x0 and x1 are any real numbers",
    ),
    (
        r"x*i = i*x",
        r"x = x0 + i*x1|x0 and x1 are any real numbers",
    ),
    (
        r"x*i = -i*x",
        r"x = x2*j + x3*k|x2 and x3 are any real numbers",
    ),
    (
        r"conj(x) = -x",
        r"x = x1*i + x2*j + x3*k|x1, x2 and x3 are any real numbers",
    ),
    (
        r"x + conj(x) = 2",
        r"x = 1 + x1*i + x2*j + x3*k|x1, x2 and x3 are any real numbers",
    ),
    (
        r"x - 2conj(x) = 1 + i + j",
        r"x = -1 + i/3 + j/3|  ~ -1 + 0.33333333333333333333333333333333i + 0.33333333333333333333333333333333j",
    ),
    (r"j*x*k + i*x = 1 - j", r"no solution"),
    (r"x/j = 1 + i", r"x = j + k"),
    (r"(x - 1)*i = j*(x + 1)", r"no solution"),
    (
        r"i*x*i + j*x*j + k*x*k = 1 + i",
        r"x = -1/3 + i|  ~ -0.33333333333333333333333333333333 + i",
    ),
    (r"x = 1 + j", r"x = 1 + j"),
    (r"3x - k = x + i", r"x = i/2 + k/2|  ~ 0.5i + 0.5k"),
    (r"x*j + y*k = 4j + 2k", r"x = -i*y + 4 + 2i"),
    (r"x*j + x*k = 4j + 2k", r"x = 3 - i"),
    (
        r"x^2 = -1",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"x^2 = 4", r"x = -2|x = 2"),
    (
        r"x^2 = -4",
        r"x = 2u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^2 + x + 1 = 0",
        r"x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075294u - 0.5|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^3 = 1",
        r"x = 1|x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075294u - 0.5|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^3 = -8",
        r"x = -2|x = sqrt(3)*u + 1, u is any unit pure quaternion|  ~ 1.7320508075688772935274463415059u + 1|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^4 = 1",
        r"x = -1|x = 1|x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^4 + 5x^2 + 4 = 0",
        r"x = 2u, u is any unit pure quaternion|x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^2 - 2x + 5 = 0",
        r"x = 2u + 1, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^3 - x^2 + x - 1 = 0",
        r"x = 1|x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"x^2 = 0", r"x = 0 (multiplicity 2)"),
    (
        r"x^6 = 1",
        r"x = -1|x = 1|x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075294u - 0.5|x = sqrt(3)*u/2 + 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075294u + 0.5|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^5 - x - 1 = 0",
        r"x ~ 1.1673039782614186842560458998548|x ~ 0.35247154603172624931794709140258u - 0.76488443360058472602982318770854, u is any unit pure quaternion|x ~ 1.0839541013177106684303444929808u + 0.18123244446987538390180023778112, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^2 + 2 = 2x",
        r"x = u + 1, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"1/x = x", r"x = -1|x = 1"),
    (
        r"x + 1/x = 1",
        r"x = sqrt(3)*u/2 + 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075294u + 0.5|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"x^2 = x", r"x = 0|x = 1"),
    (
        r"x^2 = j",
        r"x = -sqrt(2)/2 - sqrt(2)*j/2|  ~ -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485j|x = sqrt(2)/2 + sqrt(2)*j/2|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485j",
    ),
    (
        r"x^2 = 1 + j",
        r"x = -sqrt(1/2 + sqrt(2)/2) - sqrt(sqrt(2)/2 - 1/2)*j|  ~ -1.0986841134678099660398011952407 - 0.45508986056222734130435775782247j|x = sqrt(1/2 + sqrt(2)/2) + sqrt(sqrt(2)/2 - 1/2)*j|  ~ 1.0986841134678099660398011952407 + 0.45508986056222734130435775782247j",
    ),
    (
        r"x^2 = quat(0,3,4,0)",
        r"x = -sqrt(10)/2 - 3sqrt(10)*i/10 - 2sqrt(10)*j/5|  ~ -1.5811388300841896659994467722164 - 0.94868329805051379959966806332982i - 1.2649110640673517327995574177731j|x = sqrt(10)/2 + 3sqrt(10)*i/10 + 2sqrt(10)*j/5|  ~ 1.5811388300841896659994467722164 + 0.94868329805051379959966806332982i + 1.2649110640673517327995574177731j",
    ),
    (
        r"x^2 = quat(1,2,-2,1)",
        r"x = -sqrt(1/2 + sqrt(10)/2) - 2sqrt(sqrt(10)/2 - 1/2)*i/3 + 2sqrt(sqrt(10)/2 - 1/2)*j/3 - sqrt(sqrt(10)/2 - 1/2)*k/3|  ~ -1.4426152744526829201568972664345 - 0.69318550670371368923612575113965i + 0.69318550670371368923612575113965j - 0.34659275335185684461806287556982k|x = sqrt(1/2 + sqrt(10)/2) + 2sqrt(sqrt(10)/2 - 1/2)*i/3 - 2sqrt(sqrt(10)/2 - 1/2)*j/3 + sqrt(sqrt(10)/2 - 1/2)*k/3|  ~ 1.4426152744526829201568972664345 + 0.69318550670371368923612575113965i - 0.69318550670371368923612575113965j + 0.34659275335185684461806287556982k",
    ),
    (
        r"x^3 = j",
        r"x = -sqrt(3)/2 + j/2|  ~ -0.86602540378443864676372317075294 + 0.5j|x = -j|x = sqrt(3)/2 + j/2|  ~ 0.86602540378443864676372317075294 + 0.5j",
    ),
    (
        r"x^3 = 1 + i + j + k",
        r"x = -2^(1/3)*(cos(pi/9) + sqrt(3)*sin(pi/9))/2 - 2^(1/3)*(sqrt(3)*sin(pi/9) - 3cos(pi/9))*i/6 - 2^(1/3)*(sqrt(3)*sin(pi/9) - 3cos(pi/9))*j/6 - 2^(1/3)*(sqrt(3)*sin(pi/9) - 3cos(pi/9))*k/6|  ~ -0.96515551904059625308637280081316 + 0.46757383589249898572216132102779i + 0.46757383589249898572216132102779j + 0.46757383589249898572216132102779k|x = -2^(1/3)*cos(4pi/9) - 2^(1/3)*sqrt(3)*sin(4pi/9)*i/3 - 2^(1/3)*sqrt(3)*sin(4pi/9)*j/3 - 2^(1/3)*sqrt(3)*sin(4pi/9)*k/3|  ~ -0.2187829943184503520400555811351 - 0.71636467746654761940426706092048i - 0.71636467746654761940426706092048j - 0.71636467746654761940426706092048k|x = 2^(1/3)*cos(pi/9) + 2^(1/3)*sqrt(3)*sin(pi/9)*i/3 + 2^(1/3)*sqrt(3)*sin(pi/9)*j/3 + 2^(1/3)*sqrt(3)*sin(pi/9)*k/3|  ~ 1.1839385133590466051264283819483 + 0.24879084157404863368210573989269i + 0.24879084157404863368210573989269j + 0.24879084157404863368210573989269k",
    ),
    (
        r"x^2 + x = k",
        r"x = -1/2 - sqrt(1/2 + sqrt(17)/2)/2 - sqrt(sqrt(17)/2 - 1/2)*k/2|  ~ -1.3002425902201204191589098207495 - 0.62481053384382658687960444744285k|x = sqrt(1/2 + sqrt(17)/2)/2 - 1/2 + sqrt(sqrt(17)/2 - 1/2)*k/2|  ~ 0.30024259022012041915890982074952 + 0.62481053384382658687960444744285k",
    ),
    (
        r"x^2 - 2x + 2 = j - k",
        r"x = 1 - sqrt(2sqrt(3) - 2)/2 - sqrt(2)*sqrt(2 + 2sqrt(3))*j/4 + sqrt(2)*sqrt(2 + 2sqrt(3))*k/4|  ~ 0.3949996662939443908825016732787 - 0.82644582514053474004912032329269j + 0.82644582514053474004912032329269k|x = 1 + sqrt(2sqrt(3) - 2)/2 + sqrt(2)*sqrt(2 + 2sqrt(3))*j/4 - sqrt(2)*sqrt(2 + 2sqrt(3))*k/4|  ~ 1.6050003337060556091174983267213 + 0.82644582514053474004912032329269j - 0.82644582514053474004912032329269k",
    ),
    (
        r"x^4 = -1 + k",
        r"x = -2^(1/8)*cos(3pi/16) - 2^(1/8)*sin(3pi/16)*k|  ~ -0.90672404169210943229634373283255 - 0.60585363514651528639017274958085k|x = -2^(1/8)*sin(3pi/16) + 2^(1/8)*cos(3pi/16)*k|  ~ -0.60585363514651528639017274958085 + 0.90672404169210943229634373283255k|x = 2^(1/8)*sin(3pi/16) - 2^(1/8)*cos(3pi/16)*k|  ~ 0.60585363514651528639017274958085 - 0.90672404169210943229634373283255k|x = 2^(1/8)*cos(3pi/16) + 2^(1/8)*sin(3pi/16)*k|  ~ 0.90672404169210943229634373283255 + 0.60585363514651528639017274958085k",
    ),
    (
        r"x^5 + x = j",
        r"x = -sqrt(3)/2 + j/2|  ~ -0.86602540378443864676372317075294 + 0.5j|x ~ -0.74486176661974423659317042860439 - 0.87743883312334638002475444817926j|x = ((-1/3 - (11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))^2)^(1/4)*j|  ~ 0.75487766624669276004950889635853j|x ~ 0.74486176661974423659317042860439 - 0.87743883312334638002475444817926j|x = sqrt(3)/2 + j/2|  ~ 0.86602540378443864676372317075294 + 0.5j",
    ),
    (r"1/x = j", r"x = -j"),
    (
        r"x + 1/x = j",
        r"x = -(sqrt(5)/2 - 1/2)*j|  ~ -0.61803398874989484820458683436564j|x = (1/2 + sqrt(5)/2)*j|  ~ 1.6180339887498948482045868343656j",
    ),
    (
        r"x^2 = i",
        r"x = -sqrt(2)/2 - i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = sqrt(2)/2 + i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i",
    ),
    (
        r"x^3 + x = 2i - j",
        r"x ~ -0.90833405083034559573268949092798 + 0.69762850627878155944908212445159i - 0.3488142531393907797245410622258j|x ~ -1.3952570125575631188981642489032i + 0.69762850627878155944908212445159j|x ~ 0.90833405083034559573268949092798 + 0.69762850627878155944908212445159i - 0.3488142531393907797245410622258j",
    ),
    (
        r"(x^2 + 1)/(x - 1) = k",
        r"x = -sqrt(sqrt(41)/2 - 5/2)/2 + (1/2 + sqrt(5/2 + sqrt(41)/2)/2)*k|  ~ -0.41879652539044075241381626576565 + 1.6938972023080990894708965740565k|x = sqrt(sqrt(41)/2 - 5/2)/2 - (sqrt(5/2 + sqrt(41)/2)/2 - 1/2)*k|  ~ 0.41879652539044075241381626576565 - 0.69389720230809908947089657405652k",
    ),
    (r"exp(x) = j", r"x = (pi/2 + 2n*pi)*j, n is an integer"),
    (
        r"exp(x) = 1",
        r"x = 2n*pi*u, n is an integer, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"exp(x) = -1",
        r"x = u*(pi + 2n*pi), n is an integer, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"exp(x) = 2 + k",
        r"x = ln(5)/2 + (atan(1/2) + 2n*pi)*k, n is an integer",
    ),
    (
        r"exp(x) = 1 + i + j",
        r"x = ln(3)/2 + sqrt(2)*(atan(sqrt(2)) + 2n*pi)*i/2 + sqrt(2)*(atan(sqrt(2)) + 2n*pi)*j/2, n is an integer",
    ),
    (r"ln(x) = j*pi/2", r"x = j"),
    (
        r"ln(x) = 1 + k",
        r"x = e*cos(1) + e*sin(1)*k|  ~ 1.4686939399158851571389675973266 + 2.2873552871788423912081719067005k",
    ),
    (r"sin(x) = 0", r"x = n*pi, n is an integer"),
    (
        r"sin(x) = 2",
        r"x = u*ln(2 + sqrt(3)) + pi/2 + 2m*pi, m is an integer, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"cos(x) = k",
        r"x = -pi/2 + 2m*pi + asinh(1)*k, m is an integer|x = pi/2 + 2n*pi - asinh(1)*k, n is an integer",
    ),
    (
        r"cos(x) = 3",
        r"x = u*ln(3 + 2sqrt(2)) + 2m*pi, m is an integer, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"sin(x) = j",
        r"x = 2n*pi + asinh(1)*j, n is an integer|x = pi + 2m*pi - asinh(1)*j, m is an integer",
    ),
    (
        r"tan(x) = 2j",
        r"x = pi/2 + n*pi + ln(3)*j/2, n is an integer",
    ),
    (r"sinh(x) = k", r"x = (pi/2 + 2n*pi)*k, n is an integer"),
    (
        r"cosh(x) = 0",
        r"x = u*(pi/2 - 2m*pi), m is an integer, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"sqrt(x) = 1 + j", r"x = 2j"),
    (r"x^(1/2) = 2k", r"no solution"),
    (
        r"x^(3/2) = j",
        r"x = 1/2 + sqrt(3)*j/2|  ~ 0.5 + 0.86602540378443864676372317075294j",
    ),
    (r"2^x = j", r"x = (pi/2 + 2n*pi)*j/ln(2), n is an integer"),
    (
        r"e^x + e^(-x) = 2j",
        r"x = ln(3 + 2sqrt(2))/2 + (pi/2 + 2m*pi)*j, m is an integer|x = ln(17 - 12sqrt(2))/4 - (pi/2 - 2n*pi)*j, n is an integer",
    ),
    (
        r"x^2 + i*x + j = 0",
        r"x = -1/2 - i/2 + j/2 + k/2|  ~ -0.5 - 0.5i + 0.5j + 0.5k|x = 1/2 - i/2 - j/2 + k/2|  ~ 0.5 - 0.5i - 0.5j + 0.5k",
    ),
    (
        r"x^2 + x*j + k = 0",
        r"x = -1/2 - i/2 - j/2 + k/2|  ~ -0.5 - 0.5i - 0.5j + 0.5k|x = 1/2 - i/2 - j/2 - k/2|  ~ 0.5 - 0.5i - 0.5j - 0.5k",
    ),
    (r"x^2 = x*j", r"x = 0|x = j"),
    (r"x^2 = j*x", r"x = 0|x = j"),
    (
        r"x^3 + i*x = j",
        r"x ~ -0.8808438273383357247011884027713 + 0.46533216880704412441983208161383i + 0.41301567882709347798449350099886j - 0.21818791891937072571575101281653k|x ~ -0.46557123187676802665673122521994j - 0.68232780382801932736948373971105k|x ~ 0.8808438273383357247011884027713 - 0.46533216880704412441983208161383i + 0.41301567882709347798449350099886j - 0.21818791891937072571575101281653k",
    ),
    (
        r"x^2 + j*x + k = 1 + i",
        r"x = -sqrt(1/4 + sqrt(3)/2) - (2sqrt(1/4 + sqrt(3)/2) - 1)*(sqrt(3) - 1)*i/4 - j/2 + (1 + 2sqrt(1/4 + sqrt(3)/2))*(sqrt(3) - 1)*k/4|  ~ -1.0564210352811224890484565734268 - 0.20366423411292824129311799582439i - 0.5j + 0.56968963789736688805684116657732k|x = sqrt(1/4 + sqrt(3)/2) + (1 + 2sqrt(1/4 + sqrt(3)/2))*(sqrt(3) - 1)*i/4 - j/2 - (2sqrt(1/4 + sqrt(3)/2) - 1)*(sqrt(3) - 1)*k/4|  ~ 1.0564210352811224890484565734268 + 0.56968963789736688805684116657732i - 0.5j - 0.20366423411292824129311799582439k",
    ),
    (
        r"x^2 + i*x + 1 = 0",
        r"x = -i/2 - i*sqrt(5)/2|  ~ -1.6180339887498948482045868343656i|x = -i/2 + i*sqrt(5)/2|  ~ 0.61803398874989484820458683436564i",
    ),
    (r"x^2 - (i + j)*x + k = 0", r"x = j"),
    (
        r"i*x^2 + j*x + k = 0",
        r"x = -1/2 + i/2 + j/2 + k/2|  ~ -0.5 + 0.5i + 0.5j + 0.5k|x = 1/2 + i/2 - j/2 + k/2|  ~ 0.5 + 0.5i - 0.5j + 0.5k",
    ),
    (
        r"x^2*i + x*j + k = 0",
        r"x = -1/2 - i/2 - j/2 - k/2|  ~ -0.5 - 0.5i - 0.5j - 0.5k|x = 1/2 - i/2 + j/2 - k/2|  ~ 0.5 - 0.5i + 0.5j - 0.5k",
    ),
    (
        r"x^3 + j*x^2 + k*x + i = 0",
        r"x = -sqrt(2)/2 + sqrt(2)*k/2|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485k|x = i|x = sqrt(2)/2 - sqrt(2)*k/2|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485k",
    ),
    (
        r"x^3 = i*x + 1",
        r"x ~ -0.78015613730675502200913539260203 - 1.0356938354836866866503636236318i|x ~ -0.22454268478871659519199431499714 + 0.69031453455900798607336678217084i|x ~ 1.0046988220954716172011297075992 + 0.34537930092467870057699684146092i",
    ),
    (
        r"x^4 + i*x^2 + j = 0",
        r"x = -sqrt(3)/2 + sqrt(3)*i/6 + sqrt(3)*j/6 - sqrt(3)*k/6|  ~ -0.86602540378443864676372317075294 + 0.28867513459481288225457439025098i + 0.28867513459481288225457439025098j - 0.28867513459481288225457439025098k|x = -1/2 + i/2 - j/2 - k/2|  ~ -0.5 + 0.5i - 0.5j - 0.5k|x = 1/2 - i/2 + j/2 + k/2|  ~ 0.5 - 0.5i + 0.5j + 0.5k|x = sqrt(3)/2 - sqrt(3)*i/6 - sqrt(3)*j/6 + sqrt(3)*k/6|  ~ 0.86602540378443864676372317075294 - 0.28867513459481288225457439025098i - 0.28867513459481288225457439025098j + 0.28867513459481288225457439025098k",
    ),
    (
        r"x^2 + (1 + i)*x + (1 + j) = 0",
        r"x ~ -0.86243191113763599146420240057681 - 1.1897847356080651353509300602201i + 0.47518731199169062185653960626914j + 0.65555390873299094996514073839574k|x ~ -0.13756808886236400853579759942319 + 0.18978473560806513535093006022011i - 0.47518731199169062185653960626914j + 0.65555390873299094996514073839574k",
    ),
    (
        r"x^3 + x^2*j = k",
        r"x ~ -0.7276046225775956988870388978446 - 0.3843789631598499642797177335779i - 0.18023006288870946465612788838889j + 0.34116390191400966368474186985552k|x ~ -1j - 0.68232780382801932736948373971105k|x ~ 0.7276046225775956988870388978446 + 0.3843789631598499642797177335779i - 0.18023006288870946465612788838889j + 0.34116390191400966368474186985552k",
    ),
    (
        r"j*x^2 = x + k",
        r"x = -1/2 + i/2 - j/2 - k/2|  ~ -0.5 + 0.5i - 0.5j - 0.5k|x = 1/2 - i/2 - j/2 - k/2|  ~ 0.5 - 0.5i - 0.5j - 0.5k",
    ),
    (r"x^2 + i*x - x - i = 0", r"x = 1|x = -i"),
    (
        r"x^3 - (i + j + k)*x^2 + 2 = 0",
        r"x ~ -1.006845859339569465759114766225 + 0.27822010757796690836054611537419i + 0.27822010757796690836054611537419j + 0.27822010757796690836054611537419k|x ~ 0.44903211866121086179873284827861 + 1.1260539801397951164413931052033i + 1.1260539801397951164413931052033j + 1.1260539801397951164413931052033k|x ~ 0.55781374067835860396038191794635 - 0.4042740877177620248019392205775i - 0.4042740877177620248019392205775j - 0.4042740877177620248019392205775k",
    ),
    (r"x^2 - j*x - 1 - j = 0", r"x = -1|x = 1 + j"),
    (r"(x - i)*(x - j) = 0", r"x = i|x = j"),
    (r"x*(x - k) = 0", r"x = 0|x = k"),
    (
        r"x*i*x = j",
        r"x = -sqrt(2)*i/2 + sqrt(2)*j/2|  ~ -0.70710678118654752440084436210485i + 0.70710678118654752440084436210485j|x = sqrt(2)*i/2 - sqrt(2)*j/2|  ~ 0.70710678118654752440084436210485i - 0.70710678118654752440084436210485j",
    ),
    (
        r"x*i*x + x = j",
        r"x = -(-1/2 + sqrt(17)*sqrt(sqrt(17)/8 - 1/8)/4 + sqrt(sqrt(17)/8 - 1/8)/4)*i + sqrt(sqrt(17)/8 - 1/8)*j|  ~ -0.30024259022012041915890982074952i + 0.62481053384382658687960444744285j|x = (1/2 + sqrt(17)*sqrt(sqrt(17)/8 - 1/8)/4 + sqrt(sqrt(17)/8 - 1/8)/4)*i - sqrt(sqrt(17)/8 - 1/8)*j|  ~ 1.3002425902201204191589098207495i - 0.62481053384382658687960444744285j",
    ),
    (
        r"x^2*i - j*x = k",
        r"x ~ -1.0815005213161390641512147613862 - 0.27184450634603818078542798590087i + 0.58799995066033753327243605740565j + 0.5k|x ~ 1.0815005213161390641512147613862 - 0.27184450634603818078542798590087i - 0.58799995066033753327243605740565j + 0.5k",
    ),
    (
        r"x*j*x = 1",
        r"x = -sqrt(2)/2 + sqrt(2)*j/2|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485j|x = sqrt(2)/2 - sqrt(2)*j/2|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485j",
    ),
    (
        r"x*i*x = i",
        r"x = x0 + x2*j + x3*k with x0^2 + x2^2 + x3^2 = 1|x0, x2 and x3 are real numbers",
    ),
    (r"i*x^2 + x^2*j = 1", r"no solution"),
    (
        r"x*i*x*j = 1 + k",
        r"x = -sqrt(1/2 + sqrt(2)/2)*(sqrt(2) - 1)*i - sqrt(1/2 + sqrt(2)/2)*j|  ~ -0.45508986056222734130435775782247i - 1.0986841134678099660398011952407j|x = sqrt(1/2 + sqrt(2)/2)*(sqrt(2) - 1)*i + sqrt(1/2 + sqrt(2)/2)*j|  ~ 0.45508986056222734130435775782247i + 1.0986841134678099660398011952407j",
    ),
    (
        r"x^2 = conj(x)^2",
        r"x = x1*i + x2*j + x3*k|x = x0|x0, x1, x2 and x3 are any real numbers",
    ),
    (r"j/x = 1 + i", r"x = j/2 - k/2|  ~ 0.5j - 0.5k"),
    (
        r"x*conj(x) = 4",
        r"x = x0 + x1*i + x2*j + x3*k with x0^2 + x1^2 + x2^2 + x3^2 = 4|x0, x1, x2 and x3 are real numbers",
    ),
    (
        r"abs(x) = 2",
        r"x = x0 + x1*i + x2*j + x3*k with sqrt(x0^2 + x1^2 + x2^2 + x3^2) = 2|x0, x1, x2 and x3 are real numbers",
    ),
    (
        r"x^2 + conj(x) = j",
        r"x ~ -1.0906042458377742040007172072243 - 0.31434594828247115195948827128217j|x ~ 0.88660295008332304279703138905213 + 1.2933165664986180999899921912961j",
    ),
    (r"x*i*x = x", r"x = -i|x = 0"),
    (
        r"x*j*x - k*x = i",
        r"x = (1/2 + sqrt(17)*sqrt(sqrt(17)/8 - 1/8)/4 + sqrt(sqrt(17)/8 - 1/8)/4)*i - sqrt(sqrt(17)/8 - 1/8)*j|  ~ 1.3002425902201204191589098207495i - 0.62481053384382658687960444744285j|x = -(-1/2 + sqrt(17)*sqrt(sqrt(17)/8 - 1/8)/4 + sqrt(sqrt(17)/8 - 1/8)/4)*i + sqrt(sqrt(17)/8 - 1/8)*j|  ~ -0.30024259022012041915890982074952i + 0.62481053384382658687960444744285j",
    ),
    (
        r"x^2 = conj(x)",
        r"x = 0|x = 1|x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075294u - 0.5|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^2 + conj(x)^2 = 2",
        r"x = -sqrt(x1^2 + x2^2 + x3^2 + 1) + x1*i + x2*j + x3*k|x = sqrt(x1^2 + x2^2 + x3^2 + 1) + x1*i + x2*j + x3*k|x1, x2 and x3 are any real numbers",
    ),
    (
        r"x*x = conj(x)*conj(x) + i",
        r"x = i*x1 + 1/(4x1)|x1 is any real number|note: found by a search over the real components; other solutions may exist",
    ),
    (
        r"x*conj(x) + x + conj(x) = 3",
        r"x = x0 + x1*i + x2*j + x3*k with x0^2 + x1^2 + x2^2 + x3^2 + 2x0 = 3|x0, x1, x2 and x3 are real numbers",
    ),
    (
        r"re(x) = 1",
        r"x = 1 + x1*i + x2*j + x3*k|x1, x2 and x3 are any real numbers",
    ),
    (r"x*conj(x) = -1", r"no solution"),
    (
        r"x*j*x = j",
        r"x = x0 + x1*i + x3*k with x0^2 + x1^2 + x3^2 = 1|x0, x1 and x3 are real numbers",
    ),
    (
        r"x^3 = conj(x)",
        r"x = 0|x = u, u is any unit pure quaternion|x = -1|x = 1|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"x*i = conj(x)", r"x = i*x1 - x1|x1 is any real number"),
    (
        r"x^2*j = j*x^2",
        r"x = x1*i + x2*j + x3*k|x = x0 + x2*j|x0, x1, x2 and x3 are any real numbers",
    ),
    (
        r"x*i*x*i = -1",
        r"x = x0 + x2*j + x3*k with x0^2 + x2^2 + x3^2 = 1|x0, x2 and x3 are real numbers",
    ),
    (r"(x - i)*(x + i) = 0", r"x = i|x = -i"),
    (r"(x - 1)*(x - k) = 0", r"x = 1|x = k"),
    (r"x*(x - i)*(x - j) = 0", r"x = 0|x = i|x = j"),
    (
        r"x*i*conj(x) = j",
        r"x = x3 + x2*i + x2*j + x3*k with x2^2 + x3^2 = 1/2|x2 and x3 are real numbers",
    ),
    (
        r"x*i*conj(x) = 2k",
        r"x = -x2 + x3*i + x2*j + x3*k with x2^2 + x3^2 = 1|x2 and x3 are real numbers",
    ),
    (
        r"x^2 - x*i - j*x + k = 0",
        r"x = sqrt(2)/2 - sqrt(2)*k/2|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485k|x = -sqrt(2)/2 + sqrt(2)*k/2|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485k",
    ),
    (
        r"i*x*j*x = 1",
        r"x = -sqrt(2)*i/2 - sqrt(2)*j/2|  ~ -0.70710678118654752440084436210485i - 0.70710678118654752440084436210485j|x = sqrt(2)*i/2 + sqrt(2)*j/2|  ~ 0.70710678118654752440084436210485i + 0.70710678118654752440084436210485j",
    ),
    (
        r"x^2 = (1 + i)*conj(x)",
        r"x = 1/2 - sqrt(3)/2 - i/2 - i*sqrt(3)/2|  ~ -0.36602540378443864676372317075294 - 1.3660254037844386467637231707529i|x = 0|x = 1/2 + sqrt(3)/2 - i/2 + i*sqrt(3)/2|  ~ 1.3660254037844386467637231707529 + 0.36602540378443864676372317075294i|x = -1 + i",
    ),
    (
        r"x^-1 = conj(x)",
        r"x = x0 + x1*i + x2*j + x3*k with x0^2 + x1^2 + x2^2 + x3^2 = 1|x0, x1, x2 and x3 are real numbers",
    ),
    (
        r"x^2*i = i*x^2 + j",
        r"x = 1/(4x3) + x1*i + x3*k|x1 and x3 are any real numbers|note: found by a search over the real components; other solutions may exist",
    ),
    (r"abs(x)^2 = x", r"x = 0|x = 1"),
    (
        r"x^2 + 1.5x*j + k = 0",
        r"x = -0.66666666666666666666666666666667i - 1.0935921354681383791517841546645j|x = -0.66666666666666666666666666666667i - 0.40640786453186162084821584533549j",
    ),
    (r"0.5x*i - i*x = 1.5j", r"x = k"),
    (
        r"x^2 = 2.5 + j",
        r"x = -1.6113010897357532687273398691417 - 0.31030823673184379043855331200686j|x = 1.6113010897357532687273398691417 + 0.31030823673184379043855331200686j",
    ),
    (
        r"x^2 = -2.25",
        r"x = 1.5u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"1.25x^3 = j - 0.5",
        r"x = -0.89862112632594985037361265272533 + 0.34755983375993990029585591929413j|x = 0.14831491779177060211323604193107 - 0.95200864065562771896394638898363j|x = 0.75030620853417924826037661079426 + 0.6044488068956878186680904696895j",
    ),
    (
        r"exp(x) = 0.5k",
        r"x = -0.69314718055994530941723212145818 + (1.5707963267948966192313216916398 + 6.283185307179586476925286766559n)*k, n is an integer",
    ),
    (
        r"x*i*x = 0.25j + 1.5",
        r"x = -0.86011362207217006732179541756271 + 0.8719778186899466430285581877175i - 0.14335227034536167788696590292712j|x = 0.86011362207217006732179541756271 - 0.8719778186899466430285581877175i + 0.14335227034536167788696590292712j",
    ),
    (
        r"x^2 + 0.1 = 0.3x",
        r"x = 0.27838821814150109610597356494593u + 0.15, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x + y = 1 + j, x - y = i + k",
        r"x = 1/2 + i/2 + j/2 + k/2, y = 1/2 - i/2 + j/2 - k/2|  ~ x = 0.5 + 0.5i + 0.5j + 0.5k, y = 0.5 - 0.5i + 0.5j - 0.5k",
    ),
    (r"i*x + y*j = 1, x - y = k", r"no solution"),
    (r"x*y = 1, x = j", r"x = j, y = -j"),
    (r"x*i + j*y = 0, x + y = 1 + k", r"no solution"),
    (
        r"x + y*i = j, x*k - y = 1",
        r"x = 1/2 + i/2 + j/2 - k/2, y = -1/2 + i/2 - j/2 + k/2|  ~ x = 0.5 + 0.5i + 0.5j - 0.5k, y = -0.5 + 0.5i - 0.5j + 0.5k",
    ),
    (
        r"x*y = k, y*x = k",
        r"x = y3/(y0^2 + y3^2) + y0*k/(y0^2 + y3^2), y = y0 + y3*k|y0 and y3 are any real numbers|note: found by a search over the real components; other solutions may exist",
    ),
    (
        r"x^2 = j, x*y = 1",
        r"x = -sqrt(2)/2 - sqrt(2)*j/2, y = -sqrt(2)/2 + sqrt(2)*j/2|  ~ x = -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485j, y = -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485j|x = sqrt(2)/2 + sqrt(2)*j/2, y = sqrt(2)/2 - sqrt(2)*j/2|  ~ x = 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485j, y = 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485j",
    ),
    (
        r"x + y = 0, x*y = 1",
        r"x = -y1*i - y2*j - y3*k, y = y1*i + y2*j + y3*k with y1^2 + y2^2 + y3^2 = 1|y1, y2 and y3 are real numbers",
    ),
    (r"i*x + j*y = k, j*x - i*y = 1", r"no solution"),
    (r"x*i - y = 0, y*j - x = 1 + k", r"x = -k, y = -j"),
];

const QUATERNION_UNKNOWNS_AND_FORMS: &[(&str, &str)] = &[
    (
        r"solve(k^2 = -1, k)",
        r"k = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"solve(x*y = j, y)", r"y = j/x"),
    (
        r"solve(x^2 = -1, x)",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"solve(x^2 + 1, x)",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"solve([x + y = 1 + j, x - y = k], [x, y])",
        r"x = 1/2 + j/2 + k/2, y = 1/2 + j/2 - k/2|  ~ x = 0.5 + 0.5j + 0.5k, y = 0.5 + 0.5j - 0.5k",
    ),
    (r"solve([x*y = 1, x = j], [x, y])", r"x = j, y = -j"),
    (
        r"c := 1 + j; x^2 = c",
        r"1 + j|x = -sqrt(1/2 + sqrt(2)/2) - sqrt(sqrt(2)/2 - 1/2)*j|  ~ -1.0986841134678099660398011952407 - 0.45508986056222734130435775782247j|x = sqrt(1/2 + sqrt(2)/2) + sqrt(sqrt(2)/2 - 1/2)*j|  ~ 1.0986841134678099660398011952407 + 0.45508986056222734130435775782247j",
    ),
    (
        r"f(t) := t^2 + 1; f(x) = 0",
        r"f(t) := t^2 + 1|x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"x*j = j*x + a", r"no solution"),
    (r"a*x = j", r"x = j/a"),
    (
        r"a*x^2 = -1",
        r"x = u*sqrt(1/abs(a))*sin(atan2(0, -1/a)/2) - sqrt(1/abs(a))*cos(atan2(0, -1/a)/2), u is any unit pure quaternion|x = u*sqrt(1/abs(a))*sin(atan2(0, -1/a)/2) + sqrt(1/abs(a))*cos(atan2(0, -1/a)/2), u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"assume(x, real); assume(y, real); x*j + y*k = 4j + 2k",
        r"true|true|x = 4, y = 2",
    ),
    (r"assume(x, real); x*j + x*k = 4j + 2k", r"true|no solution"),
    (r"assume(x, real); x^2 = -1", r"true|no real solution"),
    (r"assume(x, real); x^x = 4", r"true|x = 2"),
    (
        r"quat(a, b, c, d)*i = j + 2k",
        r"a = 0, b = 0, c = -2, d = 1",
    ),
    (
        r"x := quat(a, b, c, d); x^2 + i*x + j = 0",
        r"a + b*i + c*j + d*k|a = -1/2, b = -1/2, c = 1/2, d = 1/2|  ~ a = -0.5, b = -0.5, c = 0.5, d = 0.5|a = 1/2, b = -1/2, c = -1/2, d = 1/2|  ~ a = 0.5, b = -0.5, c = -0.5, d = 0.5",
    ),
    (r"x = x", r"every x is a solution"),
    (r"x - x = 0", r"every x is a solution"),
    (
        r"x*i - i*x = 0",
        r"x = x0 + i*x1|x0 and x1 are any real numbers",
    ),
    (r"1 = 2", r"false"),
    (r"i*j = k", r"true"),
    (
        r"x^x = 2",
        r"error: no method for this equation: x is a quaternion here and a part of the equation takes no quaternion (after assume(x, real) it is solved for a real x)",
    ),
    (
        r"gamma(x) = 2",
        r"error: no method for this equation: x is a quaternion here and a part of the equation takes no quaternion (after assume(x, real) it is solved for a real x)",
    ),
    (
        r"floor(x) = 2",
        r"error: no method for this equation: x is a quaternion here and a part of the equation takes no quaternion (after assume(x, real) it is solved for a real x)",
    ),
    (r"diff(x^2, x) = 2", r"x = 1"),
    (r"x^2 > 1", r"x < -1 or x > 1"),
    (
        r"x^2 = [-1, 4]",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1|x = -2|x = 2",
    ),
    (
        r"x^2 = -1 ± 1",
        r"x = 0 (multiplicity 2)|x = sqrt(2)*u, u is any unit pure quaternion|  ~ 1.4142135623730950488016887242097u|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"[x, y] = [1 + j, k]", r"x = 1 + j, y = k"),
    (r"[i*x, x*j] = [k, 1]", r"no solution"),
    (
        r"re(x) = im(x)",
        r"x = x3 + x3*i + x3*j + x3*k|x3 is any real number",
    ),
    (
        r"x^2 = -1; ans",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1|x = u",
    ),
    (
        r"x^2 = 2 + j; N(ans)",
        r"x = -sqrt(1 + sqrt(5)/2) - sqrt(sqrt(5)/2 - 1)*j|  ~ -1.4553466902253548081226618397097 - 0.34356074972251246413856574391456j|x = sqrt(1 + sqrt(5)/2) + sqrt(sqrt(5)/2 - 1)*j|  ~ 1.4553466902253548081226618397097 + 0.34356074972251246413856574391456j|[x = -1.4553466902253548081226618397097 - 0.34356074972251246413856574391456j, x = 1.4553466902253548081226618397097 + 0.34356074972251246413856574391456j]",
    ),
    (
        r"x^2 = -1, y = 2",
        r"x = x1*i + x2*j + x3*k, y = 2 with x1^2 + x2^2 + x3^2 = 1|x1, x2 and x3 are real numbers",
    ),
    (r"e^x = e^j", r"x = (1 + 2n*pi)*j, n is an integer"),
    (r"x^(1/3) = j", r"no solution"),
    (
        r"x^-2 = -1",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^2 = y",
        r"x = u*sqrt(abs(y))*sin(atan2(0, y)/2) - sqrt(abs(y))*cos(atan2(0, y)/2), u is any unit pure quaternion|x = u*sqrt(abs(y))*sin(atan2(0, y)/2) + sqrt(abs(y))*cos(atan2(0, y)/2), u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"x := 5; x^2 = -1", r"5|false"),
    (r"qsolve(i*x = j, x)", r"x = -k"),
    (
        r"1 + qsolve(x = 1)",
        r"error: qsolve must be a statement of its own",
    ),
    (
        r"N(sqrt(2) + i*sqrt(3))*j",
        r"1.4142135623730950488016887242097j + 1.7320508075688772935274463415059k",
    ),
    (r"(0.5 + 0.25i)*j", r"0.5j + 0.25k"),
    (r"j*N(pi*i)", r"-3.1415926535897932384626433832795k"),
    (
        r"N(exp(i))*x*j",
        r"0.54030230586813971740093660744298x*j + 0.8414709848078965066525023216303x*k",
    ),
    (
        r"quat(a, b, c, d)^3",
        r"a*(a^2 - 3b^2 - 3c^2 - 3d^2) + b*(3a^2 - b^2 - c^2 - d^2)*i + c*(3a^2 - b^2 - c^2 - d^2)*j + d*(3a^2 - b^2 - c^2 - d^2)*k",
    ),
    (
        r"(a + b*i + c*j + d*k)*(a - b*i - c*j - d*k)",
        r"a^2 + b^2 + c^2 + d^2",
    ),
    (r"(1 + j)^(1 + k)", r"(1 + j)^(1 + k)"),
    (r"x^(1 + j)", r"x^(1 + j)"),
    (r"exp(1 + 2i + 3j + 4k)*exp(-1 - 2i - 3j - 4k)", r"1"),
    (r"(1 + 2i + 3j + 4k)^5", r"3916 + 1112i + 1668j + 2224k"),
    (r"sin(x) = 0", r"x = n*pi, n is an integer"),
    (
        r"cos(x)^2 = 1/4",
        r"x = -2pi/3 + 2p*pi, p is an integer|x = -pi/3 + 2m*pi, m is an integer|x = 2pi/3 + 2l*pi, l is an integer|x = pi/3 + 2n*pi, n is an integer",
    ),
];

const QUATERNION_EQUATIONS_WITH_OPTIONS: &[(&str, &str)] = &[
    (
        r"x^2 = 2 + j",
        r"x = -1.4553466902253548081226618397097 - 0.34356074972251246413856574391456j|x = 1.4553466902253548081226618397097 + 0.34356074972251246413856574391456j",
    ),
    (
        r"x^2 + x + 1 = 0",
        r"x = 0.86602540378443864676372317075294u - 0.5, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"i*x = j + 1/3",
        r"x = -0.33333333333333333333333333333333i - k",
    ),
    (
        r"x^3 + i*x = j",
        r"x ~ -0.8808438273383357247011884027713 + 0.46533216880704412441983208161383i + 0.41301567882709347798449350099886j - 0.21818791891937072571575101281653k|x ~ -0.46557123187676802665673122521994j - 0.68232780382801932736948373971105k|x ~ 0.8808438273383357247011884027713 - 0.46533216880704412441983208161383i + 0.41301567882709347798449350099886j - 0.21818791891937072571575101281653k",
    ),
    (
        r"x*i*x = i",
        r"x = x0 + x2*j + x3*k with x0^2 + x2^2 + x3^2 = 1|x0, x2 and x3 are real numbers",
    ),
];

const QUATERNION_EQUATIONS_EXACT: &[(&str, &str)] = &[
    (
        r"x^2 = 2 + j",
        r"x = -sqrt(1 + sqrt(5)/2) - sqrt(sqrt(5)/2 - 1)*j|x = sqrt(1 + sqrt(5)/2) + sqrt(sqrt(5)/2 - 1)*j",
    ),
    (
        r"x^2 + x + 1 = 0",
        r"x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (r"i*x = j + 1/3", r"x = -i/3 - k"),
    (
        r"x^2 + i*x + j = 0",
        r"x = -1/2 - i/2 + j/2 + k/2|x = 1/2 - i/2 - j/2 + k/2",
    ),
];

const QUATERNION_EQUATIONS_FIFTY_DIGITS: &[(&str, &str)] = &[
    (
        r"x^2 = 2 + j",
        r"x = -sqrt(1 + sqrt(5)/2) - sqrt(sqrt(5)/2 - 1)*j|  ~ -1.4553466902253548081226618397096970698548303068096 - 0.34356074972251246413856574391455856847271467681557j|x = sqrt(1 + sqrt(5)/2) + sqrt(sqrt(5)/2 - 1)*j|  ~ 1.4553466902253548081226618397096970698548303068096 + 0.34356074972251246413856574391455856847271467681557j",
    ),
    (
        r"x^2 + x + 1 = 0",
        r"x = sqrt(3)*u/2 - 1/2, u is any unit pure quaternion|  ~ 0.86602540378443864676372317075293618347140262690519u - 0.5|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"x^3 + i*x = j",
        r"x ~ -0.88084382733833572470118840277130332723060057908536 + 0.4653321688070441244198320816138349480462903041746i + 0.41301567882709347798449350099886066383594815132454j - 0.21818791891937072571575101281652776136178486789294k|x ~ -0.46557123187676802665673122521993910802557756847229j - 0.682327803828019327369483739711048256891188581898k|x ~ 0.88084382733833572470118840277130332723060057908536 - 0.4653321688070441244198320816138349480462903041746i + 0.41301567882709347798449350099886066383594815132454j - 0.21818791891937072571575101281652776136178486789294k",
    ),
    (
        r"exp(x) = 2 + k",
        r"x = ln(5)/2 + (atan(1/2) + 2n*pi)*k, n is an integer",
    ),
];

const QSOLVE_IN_OTHER_DOMAINS: &[(&str, &str)] = &[
    (r"qsolve(i*x = j, x)", r"x = -k"),
    (
        r"qsolve(x^2 = -1)",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"qsolve(x^2 + 1, x)",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"qsolve([x + y = 1 + j, x - y = k], [x, y])",
        r"x = 1/2 + j/2 + k/2, y = 1/2 + j/2 - k/2|  ~ x = 0.5 + 0.5j + 0.5k, y = 0.5 + 0.5j - 0.5k",
    ),
    (
        r"qsolve(x*i - i*x = j, x); domain()",
        r"x = x0 + x1*i + k/2|x0 and x1 are any real numbers|complex",
    ),
    (
        r"quatsolve(x^2 = j)",
        r"x = -sqrt(2)/2 - sqrt(2)*j/2|  ~ -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485j|x = sqrt(2)/2 + sqrt(2)*j/2|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485j",
    ),
    (
        r"qsolve(k^2 = -1, k)",
        r"k = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1",
    ),
    (
        r"qsolve(x^2 = -1); x^2 = -1; j^2",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1|x = -i|x = i|-1",
    ),
    (
        r"1 + qsolve(x = 1)",
        r"error: qsolve must be a statement of its own",
    ),
    (
        r"qsolve(x^3 = 1 + j)",
        r"x = -2^(2/3)/2 + 2^(2/3)*j/2|  ~ -0.79370052598409973737585281963615 + 0.79370052598409973737585281963615j|x = -2^(2/3)*(sqrt(3) - 1)/4 - 2^(2/3)*(1 + sqrt(3))*j/4|  ~ -0.29051455550725144450381318862493 - 1.0842150814913511818796660082611j|x = 2^(2/3)*(1 + sqrt(3))/4 + 2^(2/3)*(sqrt(3) - 1)*j/4|  ~ 1.0842150814913511818796660082611 + 0.29051455550725144450381318862493j",
    ),
    (
        r"qsolve(exp(x) = k, x)",
        r"x = (pi/2 + 2n*pi)*k, n is an integer",
    ),
    (
        r"domain(real); qsolve(x^2 = -4); x^2 = -4; domain()",
        r"real|x = 2u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1|no real solution|real",
    ),
];

const EQUATIONS_AND_SYSTEMS: &[(&str, &str)] = &[
    (
        r"x^5 + x = i",
        r"x = -sqrt(3)/2 + i/2|  ~ -0.86602540378443864676372317075294 + 0.5i|x = -sqrt(-1/3 + (11/54 + sqrt(69)/18)^(1/3)/2 - (sqrt(69)/18 - 11/54)^(1/3)/2 + i*sqrt(3)*((11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))/2)|  ~ -0.74486176661974423659317042860439 - 0.87743883312334638002475444817926i|x = sqrt(-1/3 - (11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))|  ~ 0.75487766624669276004950889635853i|x = sqrt(-1/3 + (11/54 + sqrt(69)/18)^(1/3)/2 - (sqrt(69)/18 - 11/54)^(1/3)/2 - i*sqrt(3)*((11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))/2)|  ~ 0.74486176661974423659317042860439 - 0.87743883312334638002475444817926i|x = sqrt(3)/2 + i/2|  ~ 0.86602540378443864676372317075294 + 0.5i",
    ),
    (
        r"x^3 + i*x + 1 = 0",
        r"x ~ -1.0046988220954716172011297075992 + 0.34537930092467870057699684146092i|x ~ 0.22454268478871659519199431499714 + 0.69031453455900798607336678217084i|x ~ 0.78015613730675502200913539260203 - 1.0356938354836866866503636236318i",
    ),
    (
        r"x^3 + (1-2i)*x^2 - (1+2i)*x - 1 = 0",
        r"x = -1|x = i (multiplicity 2)",
    ),
    (
        r"(x - i)^3*(x+2)^2*(x^2+x+i) = 0",
        r"x = -2 (multiplicity 2)|x ~ -1.3002425902201204191589098207495 + 0.62481053384382658687960444744285i|x = i (multiplicity 3)|x ~ 0.30024259022012041915890982074952 - 0.62481053384382658687960444744285i",
    ),
    (
        r"expand((x - i)^3*(x+2)^2) = 0",
        r"x = -2 (multiplicity 2)|x = i (multiplicity 3)",
    ),
    (
        r"x^5 + pi*x = 1",
        r"x ~ 0.31728634102307359674624960358708|x ~ -1.0129409449860000476445949917218 - 0.94750144152868908608952139163824i|x ~ -1.0129409449860000476445949917218 + 0.94750144152868908608952139163824i|x ~ 0.85429777447446324927147018992824 - 0.95312663109479471786891433306424i|x ~ 0.85429777447446324927147018992824 + 0.95312663109479471786891433306424i",
    ),
    (
        r"x^3 + i*pi*x = sqrt(2)",
        r"x ~ -1.2728927175323485104200806351505 + 1.4503987841615059560569214439966i|x ~ -0.027709744161057052941952308313575 - 0.44492674448065021040001897499472i|x ~ 1.3006024616934055633620329434641 - 1.0054720396808557456569024690019i",
    ),
    (
        r"x^4 + i = 0",
        r"x = -sqrt(2 + sqrt(2))/2 + i*sqrt(2 - sqrt(2))/2|  ~ -0.92387953251128675612818318939679 + 0.3826834323650897717284599840304i|x = -sqrt(2 - sqrt(2))/2 - i*sqrt(2 + sqrt(2))/2|  ~ -0.3826834323650897717284599840304 - 0.92387953251128675612818318939679i|x = sqrt(2 - sqrt(2))/2 + i*sqrt(2 + sqrt(2))/2|  ~ 0.3826834323650897717284599840304 + 0.92387953251128675612818318939679i|x = sqrt(2 + sqrt(2))/2 - i*sqrt(2 - sqrt(2))/2|  ~ 0.92387953251128675612818318939679 - 0.3826834323650897717284599840304i",
    ),
    (
        r"x^3 = i",
        r"x = -sqrt(3)/2 + i/2|  ~ -0.86602540378443864676372317075294 + 0.5i|x = -i|x = sqrt(3)/2 + i/2|  ~ 0.86602540378443864676372317075294 + 0.5i",
    ),
    (
        r"x^6 + (2+i)*x^3 + 1 = 0",
        r"x = -(-1 - sqrt(sqrt(17)/2 - 1/2)/2 - i/2 - i*sqrt(1/2 + sqrt(17)/2)/2)^(1/3)*(1 + i*sqrt(3))/2|  ~ -1.2445392348390768737806234544768 - 0.2847958509850625497977988517641i|x = (-1 + sqrt(sqrt(17)/2 - 1/2)/2 - i/2 + i*sqrt(1/2 + sqrt(17)/2)/2)^(1/3)*(-1 + i*sqrt(3))/2|  ~ -0.76352728741096373131290953558899 + 0.17472281908142397296467532162359i|x = -(-1 + sqrt(sqrt(17)/2 - 1/2)/2 - i/2 + i*sqrt(1/2 + sqrt(17)/2)/2)^(1/3)*(1 + i*sqrt(3))/2|  ~ 0.23044924376013624789688566712516 - 0.7485954369212289906148165763169i|x = (-1 - sqrt(sqrt(17)/2 - 1/2)/2 - i/2 - i*sqrt(1/2 + sqrt(17)/2)/2)^(1/3)*(-1 + i*sqrt(3))/2|  ~ 0.3756291755740668232708199850062 + 1.2202005188696191378842689279527i|x = (-1 + sqrt(sqrt(17)/2 - 1/2)/2 - i/2 + i*sqrt(1/2 + sqrt(17)/2)/2)^(1/3)|  ~ 0.53307804365082748341602386846383 + 0.5738726178398050176501412546933i|x = (-1 - sqrt(sqrt(17)/2 - 1/2)/2 - i/2 - i*sqrt(1/2 + sqrt(17)/2)/2)^(1/3)|  ~ 0.86891005926501005050980346947064 - 0.93540466788455658808647007618863i",
    ),
    (
        r"x^4 + 2i*x^2 - 1 = 0",
        r"x = -sqrt(2)/2 + i*sqrt(2)/2 (multiplicity 2)|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i|x = sqrt(2)/2 - i*sqrt(2)/2 (multiplicity 2)|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i",
    ),
    (
        r"x^7 = i",
        r"x = -cos(pi/14) + i*sin(pi/14)|  ~ -0.97492791218182360701813168299393 + 0.22252093395631440428890256449679i|x = -cos(3pi/14) - i*sin(3pi/14)|  ~ -0.78183148246802980870844452667406 - 0.62348980185873353052500488400424i|x = -cos(5pi/14) + i*sin(5pi/14)|  ~ -0.43388373911755812047576833284836 + 0.90096886790241912623610231950745i|x = -i|x = cos(5pi/14) + i*sin(5pi/14)|  ~ 0.43388373911755812047576833284836 + 0.90096886790241912623610231950745i|x = cos(3pi/14) - i*sin(3pi/14)|  ~ 0.78183148246802980870844452667406 - 0.62348980185873353052500488400424i|x = cos(pi/14) + i*sin(pi/14)|  ~ 0.97492791218182360701813168299393 + 0.22252093395631440428890256449679i",
    ),
    (r"x^3 - (1 + i)*x^2 + i*x = 0", r"x = 0|x = 1|x = i"),
    (
        r"x^4 + (1 + i)*x + 1 = 0",
        r"x ~ -1.0002455525689908286749255905468 - 0.51369349588382676990408832720094i|x ~ -0.43099128413465362530082375802251 + 0.43099128413465362530082375802251i|x ~ 0.51369349588382676990408832720094 + 1.0002455525689908286749255905468i|x ~ 0.91754334081981768407166102136834 - 0.91754334081981768407166102136834i",
    ),
    (
        r"x^5 - (2 + i)*x^4 + x - 2 - i = 0",
        r"x = -sqrt(2)/2 - i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = -sqrt(2)/2 + i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i|x = sqrt(2)/2 - i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = sqrt(2)/2 + i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i|x = 2 + i",
    ),
    (
        r"x^3 + sqrt(2)*x + pi = 0",
        r"x ~ -1.1489535452023920428734259003884|x ~ 0.57447677260119602143671295019421 - 1.5505754574182947193121547544552i|x ~ 0.57447677260119602143671295019421 + 1.5505754574182947193121547544552i",
    ),
    (
        r"x^5 - x - 1 = 0",
        r"x ~ 1.1673039782614186842560458998548|x ~ -0.76488443360058472602982318770854 - 0.35247154603172624931794709140258i|x ~ -0.76488443360058472602982318770854 + 0.35247154603172624931794709140258i|x ~ 0.18123244446987538390180023778112 - 1.0839541013177106684303444929808i|x ~ 0.18123244446987538390180023778112 + 1.0839541013177106684303444929808i",
    ),
    (
        r"x^6 - 2x^3 + x^2 + 1 = 0",
        r"x ~ -0.78015613730675502200913539260203 - 1.0356938354836866866503636236318i|x ~ -0.78015613730675502200913539260203 + 1.0356938354836866866503636236318i|x ~ -0.22454268478871659519199431499714 - 0.69031453455900798607336678217084i|x ~ -0.22454268478871659519199431499714 + 0.69031453455900798607336678217084i|x ~ 1.0046988220954716172011297075992 - 0.34537930092467870057699684146092i|x ~ 1.0046988220954716172011297075992 + 0.34537930092467870057699684146092i",
    ),
    (r"sinh(x) = i", r"x = i*pi/2 + 2i*k*pi, k is an integer"),
    (
        r"cosh(x) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cos(x)^2 = 1/4",
        r"x = -2pi/3 + 2l*pi, l is an integer|x = -pi/3 + 2n*pi, n is an integer|x = 2pi/3 + 2m*pi, m is an integer|x = pi/3 + 2k*pi, k is an integer",
    ),
    (
        r"sin(x)^2 = 1",
        r"x = -pi/2 + 2n*pi, n is an integer|x = pi/2 + 2k*pi, k is an integer",
    ),
    (
        r"tan(x)^2 = 3",
        r"x = -pi/3 + n*pi, n is an integer|x = pi/3 + k*pi, k is an integer",
    ),
    (r"cos(x) = 1", r"x = 2k*pi, k is an integer"),
    (
        r"sin(x) = 1/2",
        r"x = 5pi/6 + 2n*pi, n is an integer|x = pi/6 + 2k*pi, k is an integer",
    ),
    (r"exp(x) = 1", r"x = 2i*k*pi, k is an integer"),
    (
        r"sin(x)*cos(x) = 0",
        r"x = n*pi, n is an integer|x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"solve([x*y = 0, x*z = 0], [x, y, z])",
        r"x = 0|y = 0, z = 0|note: free: y, z (any value)|note: free: x (any value)",
    ),
    (r"x*y = 0, x + y = 1", r"x = 0, y = 1|x = 1, y = 0"),
    (
        r"solve([y/(x^2+y^2) = 0, x/(x^2+y^2) = 0], [x, y])",
        r"no solution",
    ),
    (
        r"x^2 + y^2 = 1, x*y = 0",
        r"x = -1, y = 0|x = 0, y = -1|x = 0, y = 1|x = 1, y = 0",
    ),
    (r"(x-1)*(y-2) = 0, x + y = 5", r"x = 1, y = 4|x = 3, y = 2"),
    (r"x^2 = y^2, x + y = 2", r"x = 1, y = 1"),
    (
        r"x*(x-1) = 0, y*(y-1) = 0",
        r"x = 0, y = 0|x = 0, y = 1|x = 1, y = 0|x = 1, y = 1",
    ),
    (
        r"x*y*z = 0, x + y + z = 1, x - y = 0",
        r"x = 0, y = 0, z = 1|x = 1/2, y = 1/2, z = 0|  ~ x = 0.5, y = 0.5, z = 0",
    ),
    (
        r"x^2 + y^2 = 25, x*y = 12",
        r"x = -3, y = -4|x = -4, y = -3|x = 4, y = 3|x = 3, y = 4",
    ),
    (r"x + y = 3, x - y = 1, x*y = 2", r"x = 2, y = 1"),
    (r"x^2 + y^2 = 1, x^2 + y^2 = 2", r"no solution"),
    (r"x + y = 1, x + y = 2", r"no solution"),
    (
        r"sin(x) + y = 1, x - y = 1",
        r"x = 1.1060601577062719106167372970301, y = 0.10606015770627191061673729703007|x = -10.743789480773016390181099384907 - 3.2687827563025964339361123879444i, y = -11.743789480773016390181099384907 - 3.2687827563025964339361123879444i|x = -10.743789480773016390181099384907 + 3.2687827563025964339361123879444i, y = -11.743789480773016390181099384907 + 3.2687827563025964339361123879444i|x = -4.3165396770471007402706630535392 - 2.6114255442395965953681945547882i, y = -5.3165396770471007402706630535392 - 2.6114255442395965953681945547882i|x = -4.3165396770471007402706630535392 + 2.6114255442395965953681945547882i, y = -5.3165396770471007402706630535392 + 2.6114255442395965953681945547882i|x = 3.9868981342821752127605526449077 - 1.6323448990594223732641651728579i, y = 2.9868981342821752127605526449077 - 1.6323448990594223732641651728579i|x = 3.9868981342821752127605526449077 + 1.6323448990594223732641651728579i, y = 2.9868981342821752127605526449077 + 1.6323448990594223732641651728579i|x = 10.670593855146943920308018388534 - 2.9038473037758443345039936265515i, y = 9.6705938551469439203080183885341 - 2.9038473037758443345039936265515i|x = 10.670593855146943920308018388534 + 2.9038473037758443345039936265515i, y = 9.6705938551469439203080183885341 + 2.9038473037758443345039936265515i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"exp(x) + y = 2, x - y = 0",
        r"x = 2 - lambertw(e^2), y = 2 - lambertw(e^2)|  ~ x = 0.44285440100238858314132799999934, y = 0.44285440100238858314132799999934|note: Lambert W: the real branches are given; lambertw(k, z) with other integers k gives complex solutions",
    ),
    (
        r"cos(x) = y, sin(y) = x",
        r"x = 0.69481969073078756557842007277519, y = 0.76816915673679597746208623955866|x = -2.05585949014656684183707728075 - 1.3417102622371502292364089318368i, y = -0.95280604716804636504773716202317 - 1.5765189754790050477875087089183i|x = -2.05585949014656684183707728075 + 1.3417102622371502292364089318368i, y = -0.95280604716804636504773716202317 + 1.5765189754790050477875087089183i|x = -0.26653705661098600594398729673863 - 2.5452436730627810791431962585803i, y = 6.1859644159601351079622804029283 - 1.6683113668660866129294541435585i|x = -0.26653705661098600594398729673863 + 2.5452436730627810791431962585803i, y = 6.1859644159601351079622804029283 + 1.6683113668660866129294541435585i|x = 0.1912478261206323310575136071074 - 2.9455922454415041736228947391247i, y = 9.36336287043368548118951293783 + 1.8028864430137303700894127895217i|x = 0.1912478261206323310575136071074 + 2.9455922454415041736228947391247i, y = 9.36336287043368548118951293783 - 1.8028864430137303700894127895217i|x = 0.46539176957280324669247838564308 - 1.853981289011905131233267729231i, y = 2.9230273042461563331934315676266 + 1.3976085983053748495469712750942i|x = 0.46539176957280324669247838564308 + 1.853981289011905131233267729231i, y = 2.9230273042461563331934315676266 - 1.3976085983053748495469712750942i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"exp(x)*sin(y) = 1, exp(x)*cos(y) = 1",
        r"x = 0.34657359027997265470861606072909, y = 0.78539816339744830961566084581988|x = 0.34657359027997265470861606072909 - 3.1415926535897932384626433832795i, y = -2.3561944901923449288469825374596|x = 0.34657359027997265470861606072909 + 3.1415926535897932384626433832795i, y = -2.3561944901923449288469825374596|x = 0.34657359027997265470861606072909 - 3.1415926535897932384626433832795i, y = 3.9269908169872415480783042290994|x = 0.34657359027997265470861606072909 + 3.1415926535897932384626433832795i, y = 3.9269908169872415480783042290994|x = 0.34657359027997265470861606072909, y = 7.0685834705770347865409476123789|note: found by a numeric search from many starting points: there may be other solutions",
    ),
    (r"x/(x - 1) = 2, y = x", r"x = 2, y = 2"),
    (
        r"solve([(x - 1)/(y - 1) = 0, x + y = 2], [x, y])",
        r"no solution",
    ),
    (
        r"x^2*y = 0, x + y^2 = 1",
        r"x = 0, y = -1|x = 0, y = 1|x = 1, y = 0",
    ),
];

const EQUATIONS_IN_ONE_UNKNOWN: &[(&str, &str)] = &[
    (r"sin(x) = 0", r"x = k*pi, k is an integer"),
    (r"sin(x) = 1", r"x = pi/2 + 2k*pi, k is an integer"),
    (r"sin(x) = -1", r"x = -pi/2 + 2k*pi, k is an integer"),
    (
        r"sin(x) = 1/2",
        r"x = 5pi/6 + 2n*pi, n is an integer|x = pi/6 + 2k*pi, k is an integer",
    ),
    (
        r"sin(x) = 2",
        r"x = -asin(2) + pi + 2n*pi, n is an integer|x = asin(2) + 2k*pi, k is an integer",
    ),
    (
        r"sin(x) = i",
        r"x = i*asinh(1) + 2k*pi, k is an integer|x = pi - i*asinh(1) + 2n*pi, n is an integer",
    ),
    (
        r"sin(x) = sqrt(3)",
        r"x = -asin(sqrt(3)) + pi + 2n*pi, n is an integer|x = asin(sqrt(3)) + 2k*pi, k is an integer",
    ),
    (
        r"sin(x) = -1/2",
        r"x = -pi/6 + 2k*pi, k is an integer|x = 7pi/6 + 2n*pi, n is an integer",
    ),
    (r"sin(2*x) = 1", r"x = pi/4 + k*pi, k is an integer"),
    (r"sin(x/3) = 0", r"x = 3k*pi, k is an integer"),
    (r"x*sin(x) = 0", r"x = k*pi, k is an integer"),
    (r"sin(x)/x = 0", r"x = k*pi, k is an integer other than 0"),
    (r"sin(x)*(x - 1) = 0", r"x = 1|x = k*pi, k is an integer"),
    (
        r"sin(x)^2 = 1",
        r"x = -pi/2 + 2n*pi, n is an integer|x = pi/2 + 2k*pi, k is an integer",
    ),
    (
        r"sin(x)^2 = 3",
        r"x = -asin(sqrt(3)) + 2m*pi, m is an integer|x = -asin(sqrt(3)) + pi + 2n*pi, n is an integer|x = asin(sqrt(3)) + 2k*pi, k is an integer|x = asin(sqrt(3)) + pi + 2l*pi, l is an integer",
    ),
    (
        r"x^2*sin(x) = sin(x)",
        r"x = -1|x = 1|x = k*pi, k is an integer",
    ),
    (
        r"sin(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = k*pi, k is an integer",
    ),
    (
        r"sin(x)/(x - pi) = 0",
        r"x = k*pi, k is an integer other than 1",
    ),
    (
        r"sin(x)/(x^2 - pi^2) = 0",
        r"x = k*pi, k is an integer other than -1 and 1",
    ),
    (
        r"(x - pi/2)*sin(x) = 0",
        r"x = pi/2|  ~ 1.5707963267948966192313216916398|x = k*pi, k is an integer",
    ),
    (
        r"sin(x)^2 = sin(x)",
        r"x = k*pi, k is an integer|x = pi/2 + 2n*pi, n is an integer",
    ),
    (
        r"sin(x) + 1/sin(x) = 2",
        r"x = pi/2 + 2k*pi, k is an integer",
    ),
    (r"1/sin(x) = 0", r"no solution"),
    (
        r"1/sin(x) = 2",
        r"x = 5pi/6 + 2n*pi, n is an integer|x = pi/6 + 2k*pi, k is an integer",
    ),
    (r"cos(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"cos(x) = 1", r"x = 2k*pi, k is an integer"),
    (r"cos(x) = -1", r"x = pi + 2k*pi, k is an integer"),
    (
        r"cos(x) = 1/2",
        r"x = -pi/3 + 2n*pi, n is an integer|x = pi/3 + 2k*pi, k is an integer",
    ),
    (
        r"cos(x) = 2",
        r"x = -acos(2) + 2n*pi, n is an integer|x = acos(2) + 2k*pi, k is an integer",
    ),
    (
        r"cos(x) = i",
        r"x = -pi/2 + i*asinh(1) + 2n*pi, n is an integer|x = pi/2 - i*asinh(1) + 2k*pi, k is an integer",
    ),
    (
        r"cos(x) = sqrt(3)",
        r"x = -acos(sqrt(3)) + 2n*pi, n is an integer|x = acos(sqrt(3)) + 2k*pi, k is an integer",
    ),
    (
        r"cos(x) = -1/2",
        r"x = -2pi/3 + 2n*pi, n is an integer|x = 2pi/3 + 2k*pi, k is an integer",
    ),
    (r"cos(2*x) = 1", r"x = k*pi, k is an integer"),
    (r"cos(x/3) = 0", r"x = 3pi/2 + 3k*pi, k is an integer"),
    (r"x*cos(x) = 0", r"x = 0|x = pi/2 + k*pi, k is an integer"),
    (r"cos(x)/x = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cos(x)*(x - 1) = 0",
        r"x = 1|x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cos(x)^2 = 1",
        r"x = 2k*pi, k is an integer|x = pi + 2n*pi, n is an integer",
    ),
    (
        r"cos(x)^2 = 3",
        r"x = -acos(-sqrt(3)) + 2l*pi, l is an integer|x = -acos(sqrt(3)) + 2n*pi, n is an integer|x = acos(-sqrt(3)) + 2m*pi, m is an integer|x = acos(sqrt(3)) + 2k*pi, k is an integer",
    ),
    (
        r"x^2*cos(x) = cos(x)",
        r"x = -1|x = 1|x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cos(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = pi/2 + k*pi, k is an integer",
    ),
    (r"cos(x)/(x - pi) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cos(x)/(x^2 - pi^2) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"(x - pi/2)*cos(x) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cos(x)^2 = cos(x)",
        r"x = 2n*pi, n is an integer|x = pi/2 + k*pi, k is an integer",
    ),
    (r"cos(x) + 1/cos(x) = 2", r"x = 2k*pi, k is an integer"),
    (r"1/cos(x) = 0", r"no solution"),
    (
        r"1/cos(x) = 2",
        r"x = -pi/3 + 2n*pi, n is an integer|x = pi/3 + 2k*pi, k is an integer",
    ),
    (r"tan(x) = 0", r"x = k*pi, k is an integer"),
    (r"tan(x) = 1", r"x = pi/4 + k*pi, k is an integer"),
    (r"tan(x) = -1", r"x = -pi/4 + k*pi, k is an integer"),
    (r"tan(x) = 1/2", r"x = atan(1/2) + k*pi, k is an integer"),
    (r"tan(x) = 2", r"x = atan(2) + k*pi, k is an integer"),
    (r"tan(x) = i", r"no solution"),
    (r"tan(x) = sqrt(3)", r"x = pi/3 + k*pi, k is an integer"),
    (r"tan(x) = -1/2", r"x = -atan(1/2) + k*pi, k is an integer"),
    (r"tan(2*x) = 1", r"x = pi/8 + k*pi/2, k is an integer"),
    (r"tan(x/3) = 0", r"x = 3k*pi, k is an integer"),
    (r"x*tan(x) = 0", r"x = k*pi, k is an integer"),
    (r"tan(x)/x = 0", r"x = k*pi, k is an integer other than 0"),
    (r"tan(x)*(x - 1) = 0", r"x = 1|x = k*pi, k is an integer"),
    (
        r"tan(x)^2 = 1",
        r"x = -pi/4 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (
        r"tan(x)^2 = 3",
        r"x = -pi/3 + n*pi, n is an integer|x = pi/3 + k*pi, k is an integer",
    ),
    (
        r"x^2*tan(x) = tan(x)",
        r"x = -1|x = 1|x = k*pi, k is an integer",
    ),
    (
        r"tan(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = k*pi, k is an integer",
    ),
    (
        r"tan(x)/(x - pi) = 0",
        r"x = k*pi, k is an integer other than 1",
    ),
    (
        r"tan(x)/(x^2 - pi^2) = 0",
        r"x = k*pi, k is an integer other than -1 and 1",
    ),
    (r"(x - pi/2)*tan(x) = 0", r"x = k*pi, k is an integer"),
    (
        r"tan(x)^2 = tan(x)",
        r"x = k*pi, k is an integer|x = pi/4 + n*pi, n is an integer",
    ),
    (
        r"tan(x) + 1/tan(x) = 2",
        r"x = pi/4 + k*pi, k is an integer",
    ),
    (r"1/tan(x) = 0", r"x = pi/2 + n*pi, n is an integer"),
    (r"1/tan(x) = 2", r"x = atan(1/2) + k*pi, k is an integer"),
    (r"cot(x) = 0", r"x = pi/2 + n*pi, n is an integer"),
    (r"cot(x) = 1", r"x = pi/4 + k*pi, k is an integer"),
    (r"cot(x) = -1", r"x = -pi/4 + k*pi, k is an integer"),
    (r"cot(x) = 1/2", r"x = atan(2) + k*pi, k is an integer"),
    (r"cot(x) = 2", r"x = atan(1/2) + k*pi, k is an integer"),
    (r"cot(x) = i", r"no solution"),
    (r"cot(x) = sqrt(3)", r"x = pi/6 + k*pi, k is an integer"),
    (r"cot(x) = -1/2", r"x = -atan(2) + k*pi, k is an integer"),
    (r"cot(2*x) = 1", r"x = pi/8 + k*pi/2, k is an integer"),
    (r"cot(x/3) = 0", r"x = 3pi/2 + 3n*pi, n is an integer"),
    (r"x*cot(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"cot(x)/x = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cot(x)*(x - 1) = 0",
        r"x = 1|x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cot(x)^2 = 1",
        r"x = -pi/4 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (
        r"cot(x)^2 = 3",
        r"x = -pi/6 + n*pi, n is an integer|x = pi/6 + k*pi, k is an integer",
    ),
    (
        r"x^2*cot(x) = cot(x)",
        r"x = -1|x = 1|x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cot(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = pi/2 + k*pi, k is an integer",
    ),
    (r"cot(x)/(x - pi) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cot(x)/(x^2 - pi^2) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"(x - pi/2)*cot(x) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cot(x)^2 = cot(x)",
        r"x = pi/2 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (
        r"cot(x) + 1/cot(x) = 2",
        r"x = pi/4 + k*pi, k is an integer",
    ),
    (r"1/cot(x) = 0", r"x = k*pi, k is an integer"),
    (r"1/cot(x) = 2", r"x = atan(2) + k*pi, k is an integer"),
    (
        r"sinh(x) = 0",
        r"x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x) = 1",
        r"x = -asinh(1) + i*pi + 2i*n*pi, n is an integer|x = asinh(1) + 2i*k*pi, k is an integer",
    ),
    (
        r"sinh(x) = -1",
        r"x = -asinh(1) + 2i*k*pi, k is an integer|x = asinh(1) + i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x) = 1/2",
        r"x = -asinh(1/2) + i*pi + 2i*n*pi, n is an integer|x = asinh(1/2) + 2i*k*pi, k is an integer",
    ),
    (
        r"sinh(x) = 2",
        r"x = -asinh(2) + i*pi + 2i*n*pi, n is an integer|x = asinh(2) + 2i*k*pi, k is an integer",
    ),
    (r"sinh(x) = i", r"x = i*pi/2 + 2i*k*pi, k is an integer"),
    (
        r"sinh(x) = sqrt(3)",
        r"x = -asinh(sqrt(3)) + i*pi + 2i*n*pi, n is an integer|x = asinh(sqrt(3)) + 2i*k*pi, k is an integer",
    ),
    (
        r"sinh(x) = -1/2",
        r"x = -asinh(1/2) + 2i*k*pi, k is an integer|x = asinh(1/2) + i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(2*x) = 1",
        r"x = -asinh(1)/2 + i*pi/2 + i*n*pi, n is an integer|x = asinh(1)/2 + i*k*pi, k is an integer",
    ),
    (
        r"sinh(x/3) = 0",
        r"x = 3i*pi + 6i*n*pi, n is an integer|x = 6i*k*pi, k is an integer",
    ),
    (
        r"x*sinh(x) = 0",
        r"x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)/x = 0",
        r"x = 2i*k*pi, k is an integer other than 0|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)*(x - 1) = 0",
        r"x = 1|x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)^2 = 1",
        r"x = -asinh(1) + 2i*m*pi, m is an integer|x = -asinh(1) + i*pi + 2i*n*pi, n is an integer|x = asinh(1) + 2i*k*pi, k is an integer|x = asinh(1) + i*pi + 2i*l*pi, l is an integer",
    ),
    (
        r"sinh(x)^2 = 3",
        r"x = -asinh(sqrt(3)) + 2i*m*pi, m is an integer|x = -asinh(sqrt(3)) + i*pi + 2i*n*pi, n is an integer|x = asinh(sqrt(3)) + 2i*k*pi, k is an integer|x = asinh(sqrt(3)) + i*pi + 2i*l*pi, l is an integer",
    ),
    (
        r"x^2*sinh(x) = sinh(x)",
        r"x = -1|x = 1|x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)/(x - pi) = 0",
        r"x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)/(x^2 - pi^2) = 0",
        r"x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"(x - pi/2)*sinh(x) = 0",
        r"x = pi/2|  ~ 1.5707963267948966192313216916398|x = 2i*k*pi, k is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x)^2 = sinh(x)",
        r"x = -asinh(1) + i*pi + 2i*l*pi, l is an integer|x = 2i*k*pi, k is an integer|x = asinh(1) + 2i*m*pi, m is an integer|x = i*pi + 2i*n*pi, n is an integer",
    ),
    (
        r"sinh(x) + 1/sinh(x) = 2",
        r"x = -asinh(1) + i*pi + 2i*n*pi, n is an integer|x = asinh(1) + 2i*k*pi, k is an integer",
    ),
    (r"1/sinh(x) = 0", r"no solution"),
    (
        r"1/sinh(x) = 2",
        r"x = -asinh(1/2) + i*pi + 2i*n*pi, n is an integer|x = asinh(1/2) + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (r"cosh(x) = 1", r"x = 2i*k*pi, k is an integer"),
    (
        r"cosh(x) = -1",
        r"x = -acosh(-1) + 2i*n*pi, n is an integer|x = acosh(-1) + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x) = 1/2",
        r"x = -acosh(1/2) + 2i*n*pi, n is an integer|x = acosh(1/2) + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x) = 2",
        r"x = -acosh(2) + 2i*n*pi, n is an integer|x = acosh(2) + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x) = i",
        r"x = -acosh(i) + 2i*n*pi, n is an integer|x = acosh(i) + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x) = sqrt(3)",
        r"x = -acosh(sqrt(3)) + 2i*n*pi, n is an integer|x = acosh(sqrt(3)) + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x) = -1/2",
        r"x = -acosh(-1/2) + 2i*n*pi, n is an integer|x = acosh(-1/2) + 2i*k*pi, k is an integer",
    ),
    (r"cosh(2*x) = 1", r"x = i*k*pi, k is an integer"),
    (
        r"cosh(x/3) = 0",
        r"x = -3i*pi/2 + 6i*n*pi, n is an integer|x = 3i*pi/2 + 6i*k*pi, k is an integer",
    ),
    (
        r"x*cosh(x) = 0",
        r"x = 0|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)/x = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)*(x - 1) = 0",
        r"x = 1|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)^2 = 1",
        r"x = -acosh(-1) + 2i*l*pi, l is an integer|x = 2i*k*pi, k is an integer|x = acosh(-1) + 2i*m*pi, m is an integer",
    ),
    (
        r"cosh(x)^2 = 3",
        r"x = -acosh(-sqrt(3)) + 2i*l*pi, l is an integer|x = -acosh(sqrt(3)) + 2i*n*pi, n is an integer|x = acosh(-sqrt(3)) + 2i*m*pi, m is an integer|x = acosh(sqrt(3)) + 2i*k*pi, k is an integer",
    ),
    (
        r"x^2*cosh(x) = cosh(x)",
        r"x = -1|x = 1|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)/(x - pi) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)/(x^2 - pi^2) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"(x - pi/2)*cosh(x) = 0",
        r"x = pi/2|  ~ 1.5707963267948966192313216916398|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"cosh(x)^2 = cosh(x)",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = 2i*m*pi (multiplicity 2), m is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (r"cosh(x) + 1/cosh(x) = 2", r"x = 2i*k*pi, k is an integer"),
    (r"1/cosh(x) = 0", r"no solution"),
    (
        r"1/cosh(x) = 2",
        r"x = -acosh(1/2) + 2i*n*pi, n is an integer|x = acosh(1/2) + 2i*k*pi, k is an integer",
    ),
    (r"tanh(x) = 0", r"x = i*k*pi, k is an integer"),
    (r"tanh(x) = 1", r"no solution"),
    (r"tanh(x) = -1", r"no solution"),
    (
        r"tanh(x) = 1/2",
        r"x = atanh(1/2) + i*k*pi, k is an integer",
    ),
    (r"tanh(x) = 2", r"x = atanh(2) + i*k*pi, k is an integer"),
    (r"tanh(x) = i", r"x = i*pi/4 + i*k*pi, k is an integer"),
    (
        r"tanh(x) = sqrt(3)",
        r"x = atanh(sqrt(3)) + i*k*pi, k is an integer",
    ),
    (
        r"tanh(x) = -1/2",
        r"x = -atanh(1/2) + i*k*pi, k is an integer",
    ),
    (r"tanh(2*x) = 1", r"no solution"),
    (r"tanh(x/3) = 0", r"x = 3i*k*pi, k is an integer"),
    (r"x*tanh(x) = 0", r"x = i*k*pi, k is an integer"),
    (
        r"tanh(x)/x = 0",
        r"x = i*k*pi, k is an integer other than 0",
    ),
    (r"tanh(x)*(x - 1) = 0", r"x = 1|x = i*k*pi, k is an integer"),
    (r"tanh(x)^2 = 1", r"no solution"),
    (
        r"tanh(x)^2 = 3",
        r"x = -atanh(sqrt(3)) + i*n*pi, n is an integer|x = atanh(sqrt(3)) + i*k*pi, k is an integer",
    ),
    (
        r"x^2*tanh(x) = tanh(x)",
        r"x = -1|x = 1|x = i*k*pi, k is an integer",
    ),
    (
        r"tanh(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = i*k*pi, k is an integer",
    ),
    (r"tanh(x)/(x - pi) = 0", r"x = i*k*pi, k is an integer"),
    (r"tanh(x)/(x^2 - pi^2) = 0", r"x = i*k*pi, k is an integer"),
    (
        r"(x - pi/2)*tanh(x) = 0",
        r"x = pi/2|  ~ 1.5707963267948966192313216916398|x = i*k*pi, k is an integer",
    ),
    (r"tanh(x)^2 = tanh(x)", r"x = i*k*pi, k is an integer"),
    (r"tanh(x) + 1/tanh(x) = 2", r"no solution"),
    (
        r"1/tanh(x) = 0",
        r"x = -i*pi/2 + 2i*m*pi, m is an integer|x = i*pi/2 + 2i*n*pi, n is an integer",
    ),
    (
        r"1/tanh(x) = 2",
        r"x = atanh(1/2) + i*k*pi, k is an integer",
    ),
    (
        r"coth(x) = 0",
        r"x = -i*pi/2 + 2i*m*pi, m is an integer|x = i*pi/2 + 2i*n*pi, n is an integer",
    ),
    (r"coth(x) = 1", r"no solution"),
    (r"coth(x) = -1", r"no solution"),
    (r"coth(x) = 1/2", r"x = atanh(2) + i*k*pi, k is an integer"),
    (r"coth(x) = 2", r"x = atanh(1/2) + i*k*pi, k is an integer"),
    (r"coth(x) = i", r"x = -i*pi/4 + i*k*pi, k is an integer"),
    (
        r"coth(x) = sqrt(3)",
        r"x = atanh(sqrt(3)/3) + i*k*pi, k is an integer",
    ),
    (
        r"coth(x) = -1/2",
        r"x = -atanh(2) + i*k*pi, k is an integer",
    ),
    (r"coth(2*x) = 1", r"no solution"),
    (
        r"coth(x/3) = 0",
        r"x = -3i*pi/2 + 6i*m*pi, m is an integer|x = 3i*pi/2 + 6i*n*pi, n is an integer",
    ),
    (
        r"x*coth(x) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"coth(x)/x = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"coth(x)*(x - 1) = 0",
        r"x = 1|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (r"coth(x)^2 = 1", r"no solution"),
    (
        r"coth(x)^2 = 3",
        r"x = -atanh(sqrt(3)/3) + i*n*pi, n is an integer|x = atanh(sqrt(3)/3) + i*k*pi, k is an integer",
    ),
    (
        r"x^2*coth(x) = coth(x)",
        r"x = -1|x = 1|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"coth(x)*(x^2 + 1) = 0",
        r"x = -i|x = i|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"coth(x)/(x - pi) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"coth(x)/(x^2 - pi^2) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"(x - pi/2)*coth(x) = 0",
        r"x = pi/2|  ~ 1.5707963267948966192313216916398|x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"coth(x)^2 = coth(x)",
        r"x = -i*pi/2 + 2i*m*pi, m is an integer|x = i*pi/2 + 2i*n*pi, n is an integer",
    ),
    (r"coth(x) + 1/coth(x) = 2", r"no solution"),
    (r"1/coth(x) = 0", r"x = i*k*pi, k is an integer"),
    (r"1/coth(x) = 2", r"x = atanh(2) + i*k*pi, k is an integer"),
    (r"sec(x) = 0", r"no solution"),
    (r"sec(x) = 1", r"x = 2k*pi, k is an integer"),
    (r"sec(x) = -1", r"x = pi + 2k*pi, k is an integer"),
    (
        r"sec(x) = 1/2",
        r"x = -acos(2) + 2n*pi, n is an integer|x = acos(2) + 2k*pi, k is an integer",
    ),
    (
        r"sec(x) = 2",
        r"x = -pi/3 + 2n*pi, n is an integer|x = pi/3 + 2k*pi, k is an integer",
    ),
    (
        r"sec(x) = i",
        r"x = -pi/2 - i*asinh(1) + 2n*pi, n is an integer|x = pi/2 + i*asinh(1) + 2k*pi, k is an integer",
    ),
    (
        r"sec(x) = sqrt(3)",
        r"x = -acos(sqrt(3)/3) + 2n*pi, n is an integer|x = acos(sqrt(3)/3) + 2k*pi, k is an integer",
    ),
    (
        r"sec(x) = -1/2",
        r"x = -acos(-2) + 2n*pi, n is an integer|x = acos(-2) + 2k*pi, k is an integer",
    ),
    (r"sec(2*x) = 1", r"x = k*pi, k is an integer"),
    (r"sec(x/3) = 0", r"no solution"),
    (r"x*sec(x) = 0", r"x = 0"),
    (r"sec(x)/x = 0", r"no solution"),
    (r"sec(x)*(x - 1) = 0", r"x = 1"),
    (
        r"sec(x)^2 = 1",
        r"x = 2k*pi, k is an integer|x = pi + 2n*pi, n is an integer",
    ),
    (
        r"sec(x)^2 = 3",
        r"x = -acos(-sqrt(3)/3) + 2l*pi, l is an integer|x = -acos(sqrt(3)/3) + 2n*pi, n is an integer|x = acos(-sqrt(3)/3) + 2m*pi, m is an integer|x = acos(sqrt(3)/3) + 2k*pi, k is an integer",
    ),
    (r"x^2*sec(x) = sec(x)", r"x = -1|x = 1"),
    (r"sec(x)*(x^2 + 1) = 0", r"x = -i|x = i"),
    (r"sec(x)/(x - pi) = 0", r"no solution"),
    (r"sec(x)/(x^2 - pi^2) = 0", r"no solution"),
    (r"(x - pi/2)*sec(x) = 0", r"no solution"),
    (r"sec(x)^2 = sec(x)", r"x = 2k*pi, k is an integer"),
    (r"sec(x) + 1/sec(x) = 2", r"x = 2k*pi, k is an integer"),
    (r"1/sec(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"1/sec(x) = 2",
        r"x = -acos(2) + 2n*pi, n is an integer|x = acos(2) + 2k*pi, k is an integer",
    ),
    (r"csc(x) = 0", r"no solution"),
    (r"csc(x) = 1", r"x = pi/2 + 2k*pi, k is an integer"),
    (r"csc(x) = -1", r"x = -pi/2 + 2k*pi, k is an integer"),
    (
        r"csc(x) = 1/2",
        r"x = -asin(2) + pi + 2n*pi, n is an integer|x = asin(2) + 2k*pi, k is an integer",
    ),
    (
        r"csc(x) = 2",
        r"x = 5pi/6 + 2n*pi, n is an integer|x = pi/6 + 2k*pi, k is an integer",
    ),
    (
        r"csc(x) = i",
        r"x = -i*asinh(1) + 2k*pi, k is an integer|x = pi + i*asinh(1) + 2n*pi, n is an integer",
    ),
    (
        r"csc(x) = sqrt(3)",
        r"x = -asin(sqrt(3)/3) + pi + 2n*pi, n is an integer|x = asin(sqrt(3)/3) + 2k*pi, k is an integer",
    ),
    (
        r"csc(x) = -1/2",
        r"x = -asin(2) + 2k*pi, k is an integer|x = asin(2) + pi + 2n*pi, n is an integer",
    ),
    (r"csc(2*x) = 1", r"x = pi/4 + k*pi, k is an integer"),
    (r"csc(x/3) = 0", r"no solution"),
    (r"x*csc(x) = 0", r"no solution"),
    (r"csc(x)/x = 0", r"no solution"),
    (r"csc(x)*(x - 1) = 0", r"x = 1"),
    (
        r"csc(x)^2 = 1",
        r"x = -pi/2 + 2n*pi, n is an integer|x = pi/2 + 2k*pi, k is an integer",
    ),
    (
        r"csc(x)^2 = 3",
        r"x = -asin(sqrt(3)/3) + 2m*pi, m is an integer|x = -asin(sqrt(3)/3) + pi + 2n*pi, n is an integer|x = asin(sqrt(3)/3) + 2k*pi, k is an integer|x = asin(sqrt(3)/3) + pi + 2l*pi, l is an integer",
    ),
    (r"x^2*csc(x) = csc(x)", r"x = -1|x = 1"),
    (r"csc(x)*(x^2 + 1) = 0", r"x = -i|x = i"),
    (r"csc(x)/(x - pi) = 0", r"no solution"),
    (r"csc(x)/(x^2 - pi^2) = 0", r"no solution"),
    (
        r"(x - pi/2)*csc(x) = 0",
        r"x = pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (r"csc(x)^2 = csc(x)", r"x = pi/2 + 2k*pi, k is an integer"),
    (
        r"csc(x) + 1/csc(x) = 2",
        r"x = pi/2 + 2k*pi, k is an integer",
    ),
    (r"1/csc(x) = 0", r"x = k*pi, k is an integer"),
    (
        r"1/csc(x) = 2",
        r"x = -asin(2) + pi + 2n*pi, n is an integer|x = asin(2) + 2k*pi, k is an integer",
    ),
    (
        r"sin(x)*sin(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"sin(x)/sin(x) = 0", r"false"),
    (r"sin(2*x)/sin(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"sin(x)/sin(x) = 1", r"true"),
    (
        r"sin(x)*cos(x) = 0",
        r"x = n*pi, n is an integer|x = pi/2 + k*pi, k is an integer",
    ),
    (r"sin(x)/cos(x) = 0", r"x = k*pi, k is an integer"),
    (r"sin(2*x)/cos(x) = 0", r"x = k*pi, k is an integer"),
    (r"sin(x)/cos(x) = 1", r"x = pi/4 + k*pi, k is an integer"),
    (
        r"sin(x)*tan(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"sin(x)/tan(x) = 0", r"x = pi/2 + n*pi, n is an integer"),
    (r"sin(2*x)/tan(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"sin(x)/tan(x) = 1", r"no solution"),
    (r"sin(x)*cot(x) = 0", r"x = pi/2 + n*pi, n is an integer"),
    (
        r"sin(x)/cot(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"sin(2*x)/cot(x) = 0", r"x = k*pi, k is an integer"),
    (
        r"sin(x)/cot(x) = 1",
        r"x = -atan(sqrt(2sqrt(5) - 2)/4 + sqrt(5)*sqrt(2sqrt(5) - 2)/4) + 2m*pi, m is an integer|x = atan(sqrt(2sqrt(5) - 2)/4 + sqrt(5)*sqrt(2sqrt(5) - 2)/4) + 2l*pi, l is an integer|x = pi - i*ln(2 + sqrt(2 + 2sqrt(5))/2 + sqrt(5) + sqrt(5)*sqrt(2 + 2sqrt(5))/2)/2 + 2k*pi, k is an integer|x = pi - i*ln(2 - sqrt(2 + 2sqrt(5))/2 + sqrt(5) - sqrt(5)*sqrt(2 + 2sqrt(5))/2)/2 + 2n*pi, n is an integer",
    ),
    (
        r"cos(x)*sin(x) = 0",
        r"x = n*pi, n is an integer|x = pi/2 + k*pi, k is an integer",
    ),
    (r"cos(x)/sin(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cos(2*x)/sin(x) = 0",
        r"x = pi/4 + k*pi/2, k is an integer",
    ),
    (r"cos(x)/sin(x) = 1", r"x = pi/4 + k*pi, k is an integer"),
    (
        r"cos(x)*cos(x) = 0",
        r"x = pi/2 + k*pi (multiplicity 2), k is an integer",
    ),
    (r"cos(x)/cos(x) = 0", r"false"),
    (
        r"cos(2*x)/cos(x) = 0",
        r"x = pi/4 + k*pi/2, k is an integer",
    ),
    (r"cos(x)/cos(x) = 1", r"true"),
    (r"cos(x)*tan(x) = 0", r"x = n*pi, n is an integer"),
    (r"cos(x)/tan(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cos(2*x)/tan(x) = 0",
        r"x = pi/2 + n*pi, n is an integer|x = pi/4 + k*pi/2, k is an integer",
    ),
    (
        r"cos(x)/tan(x) = 1",
        r"x = -atan(sqrt(3/2 - sqrt(5)/2)/sqrt(sqrt(5)/2 - 1/2)) + pi + 2m*pi, m is an integer|x = -pi/2 - i*ln(2 + sqrt(32 + 16sqrt(5))/2 + sqrt(5))/2 + 2k*pi, k is an integer|x = -pi/2 - i*ln(2 - sqrt(32 + 16sqrt(5))/2 + sqrt(5))/2 + 2n*pi, n is an integer|x = atan(sqrt(3/2 - sqrt(5)/2)/sqrt(sqrt(5)/2 - 1/2)) + 2l*pi, l is an integer",
    ),
    (r"cos(x)*cot(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"cos(x)/cot(x) = 0", r"x = n*pi, n is an integer"),
    (
        r"cos(2*x)/cot(x) = 0",
        r"x = n*pi, n is an integer|x = pi/4 + k*pi/2, k is an integer",
    ),
    (r"cos(x)/cot(x) = 1", r"no solution"),
    (
        r"tan(x)*sin(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"tan(x)/sin(x) = 0", r"no solution"),
    (r"tan(2*x)/sin(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"tan(x)/sin(x) = 1", r"no solution"),
    (r"tan(x)*cos(x) = 0", r"x = n*pi, n is an integer"),
    (r"tan(x)/cos(x) = 0", r"x = k*pi, k is an integer"),
    (r"tan(2*x)/cos(x) = 0", r"x = k*pi, k is an integer"),
    (
        r"tan(x)/cos(x) = 1",
        r"x = -atan(sqrt(3/2 - sqrt(5)/2)/sqrt(sqrt(5)/2 - 1/2)) + pi + 2m*pi, m is an integer|x = -pi/2 - i*ln(2 + sqrt(32 + 16sqrt(5))/2 + sqrt(5))/2 + 2k*pi, k is an integer|x = -pi/2 - i*ln(2 - sqrt(32 + 16sqrt(5))/2 + sqrt(5))/2 + 2n*pi, n is an integer|x = atan(sqrt(3/2 - sqrt(5)/2)/sqrt(sqrt(5)/2 - 1/2)) + 2l*pi, l is an integer",
    ),
    (
        r"tan(x)*tan(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"tan(x)/tan(x) = 0", r"false"),
    (r"tan(2*x)/tan(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"tan(x)/tan(x) = 1", r"true"),
    (r"tan(x)*cot(x) = 0", r"false"),
    (
        r"tan(x)/cot(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"tan(2*x)/cot(x) = 0", r"x = n*pi, n is an integer"),
    (
        r"tan(x)/cot(x) = 1",
        r"x = -pi/4 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (r"cot(x)*sin(x) = 0", r"x = pi/2 + n*pi, n is an integer"),
    (r"cot(x)/sin(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"cot(2*x)/sin(x) = 0",
        r"x = pi/4 + k*pi/2, k is an integer",
    ),
    (
        r"cot(x)/sin(x) = 1",
        r"x = -atan(sqrt(2sqrt(5) - 2)/4 + sqrt(5)*sqrt(2sqrt(5) - 2)/4) + 2m*pi, m is an integer|x = atan(sqrt(2sqrt(5) - 2)/4 + sqrt(5)*sqrt(2sqrt(5) - 2)/4) + 2l*pi, l is an integer|x = pi - i*ln(2 + sqrt(2 + 2sqrt(5))/2 + sqrt(5) + sqrt(5)*sqrt(2 + 2sqrt(5))/2)/2 + 2k*pi, k is an integer|x = pi - i*ln(2 - sqrt(2 + 2sqrt(5))/2 + sqrt(5) - sqrt(5)*sqrt(2 + 2sqrt(5))/2)/2 + 2n*pi, n is an integer",
    ),
    (r"cot(x)*cos(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"cot(x)/cos(x) = 0", r"no solution"),
    (
        r"cot(2*x)/cos(x) = 0",
        r"x = pi/4 + k*pi/2, k is an integer",
    ),
    (r"cot(x)/cos(x) = 1", r"no solution"),
    (r"cot(x)*tan(x) = 0", r"false"),
    (
        r"cot(x)/tan(x) = 0",
        r"x = pi/2 + m*pi (multiplicity 2), m is an integer",
    ),
    (
        r"cot(2*x)/tan(x) = 0",
        r"x = pi/4 + n*pi/2, n is an integer",
    ),
    (
        r"cot(x)/tan(x) = 1",
        r"x = -pi/4 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (
        r"cot(x)*cot(x) = 0",
        r"x = pi/2 + m*pi (multiplicity 2), m is an integer",
    ),
    (r"cot(x)/cot(x) = 0", r"false"),
    (
        r"cot(2*x)/cot(x) = 0",
        r"x = pi/4 + n*pi/2, n is an integer",
    ),
    (r"cot(x)/cot(x) = 1", r"true"),
    (
        r"sin(2*x)/sin(1*x) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (
        r"cos(2*x)/cos(1*x) = 0",
        r"x = pi/4 + k*pi/2, k is an integer",
    ),
    (r"sin(2*x)/cos(1*x) = 0", r"x = k*pi, k is an integer"),
    (
        r"sin(2*x) = sin(1*x)",
        r"x = -pi/3 + 2m*pi, m is an integer|x = 2k*pi, k is an integer|x = pi + 2n*pi, n is an integer|x = pi/3 + 2l*pi, l is an integer",
    ),
    (
        r"cos(2*x) = cos(1*x)",
        r"x = -2pi/3 + 2n*pi, n is an integer|x = 2k*pi (multiplicity 2), k is an integer|x = 2pi/3 + 2m*pi, m is an integer",
    ),
    (
        r"sin(3*x)/sin(1*x) = 0",
        r"x = 2pi/3 + n*pi, n is an integer|x = pi/3 + k*pi, k is an integer",
    ),
    (
        r"cos(3*x)/cos(1*x) = 0",
        r"x = 5pi/6 + n*pi, n is an integer|x = pi/6 + k*pi, k is an integer",
    ),
    (r"sin(3*x)/cos(1*x) = 0", r"x = k*pi/3, k is an integer"),
    (
        r"sin(3*x) = sin(1*x)",
        r"x = k*pi, k is an integer|x = pi/4 + n*pi/2, n is an integer",
    ),
    (
        r"cos(3*x) = cos(1*x)",
        r"x = k*pi (multiplicity 2), k is an integer|x = pi/2 + n*pi, n is an integer",
    ),
    (
        r"sin(4*x)/sin(2*x) = 0",
        r"x = pi/4 + k*pi/2, k is an integer",
    ),
    (
        r"cos(4*x)/cos(2*x) = 0",
        r"x = pi/8 + k*pi/4, k is an integer",
    ),
    (r"sin(4*x)/cos(2*x) = 0", r"x = k*pi/2, k is an integer"),
    (
        r"sin(4*x) = sin(2*x)",
        r"x = -pi/6 + m*pi, m is an integer|x = k*pi, k is an integer|x = pi/2 + n*pi, n is an integer|x = pi/6 + l*pi, l is an integer",
    ),
    (
        r"cos(4*x) = cos(2*x)",
        r"x = -pi/3 + n*pi, n is an integer|x = k*pi (multiplicity 2), k is an integer|x = pi/3 + m*pi, m is an integer",
    ),
    (r"sin(1*x)/sin(2*x) = 0", r"no solution"),
    (
        r"cos(1*x)/cos(2*x) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (r"sin(1*x)/cos(2*x) = 0", r"x = k*pi, k is an integer"),
    (
        r"sin(1*x) = sin(2*x)",
        r"x = -pi/3 + 2m*pi, m is an integer|x = 2k*pi, k is an integer|x = pi + 2n*pi, n is an integer|x = pi/3 + 2l*pi, l is an integer",
    ),
    (
        r"cos(1*x) = cos(2*x)",
        r"x = -2pi/3 + 2n*pi, n is an integer|x = 2k*pi (multiplicity 2), k is an integer|x = 2pi/3 + 2m*pi, m is an integer",
    ),
    (
        r"sin(3*x)/sin(2*x) = 0",
        r"x = 2pi/3 + n*pi, n is an integer|x = pi/3 + k*pi, k is an integer",
    ),
    (
        r"cos(3*x)/cos(2*x) = 0",
        r"x = pi/6 + k*pi/3, k is an integer",
    ),
    (r"sin(3*x)/cos(2*x) = 0", r"x = k*pi/3, k is an integer"),
    (
        r"sin(3*x) = sin(2*x)",
        r"x = -atan(sqrt(10 + 2sqrt(5))/4 + sqrt(5)*sqrt(10 + 2sqrt(5))/4) + pi + 2l*pi, l is an integer|x = -atan(sqrt(10 - 2sqrt(5))/4 - sqrt(5)*sqrt(10 - 2sqrt(5))/4) + 2m*pi, m is an integer|x = 2k*pi, k is an integer|x = atan(sqrt(10 + 2sqrt(5))/4 + sqrt(5)*sqrt(10 + 2sqrt(5))/4) - pi + 2p*pi, p is an integer|x = atan(sqrt(10 - 2sqrt(5))/4 - sqrt(5)*sqrt(10 - 2sqrt(5))/4) + 2pi*q, q is an integer|x = pi + 2n*pi, n is an integer",
    ),
    (
        r"cos(3*x) = cos(2*x)",
        r"x = -atan(sqrt(10 + 2sqrt(5))/4 + sqrt(5)*sqrt(10 + 2sqrt(5))/4) + 2p*pi, p is an integer|x = -atan(sqrt(10 - 2sqrt(5))/4 - sqrt(5)*sqrt(10 - 2sqrt(5))/4) - pi + 2l*pi, l is an integer|x = 2k*pi (multiplicity 2), k is an integer|x = atan(sqrt(10 + 2sqrt(5))/4 + sqrt(5)*sqrt(10 + 2sqrt(5))/4) + 2n*pi, n is an integer|x = atan(sqrt(10 - 2sqrt(5))/4 - sqrt(5)*sqrt(10 - 2sqrt(5))/4) + pi + 2m*pi, m is an integer",
    ),
    (
        r"sin(6*x)/sin(3*x) = 0",
        r"x = pi/6 + k*pi/3, k is an integer",
    ),
    (
        r"cos(6*x)/cos(3*x) = 0",
        r"x = pi/12 + k*pi/6, k is an integer",
    ),
    (r"sin(6*x)/cos(3*x) = 0", r"x = k*pi/3, k is an integer"),
    (
        r"sin(6*x) = sin(3*x)",
        r"x = -pi/9 + 2m*pi/3, m is an integer|x = 2k*pi/3, k is an integer|x = pi/3 + 2n*pi/3, n is an integer|x = pi/9 + 2l*pi/3, l is an integer",
    ),
    (
        r"cos(6*x) = cos(3*x)",
        r"x = -2pi/9 + 2n*pi/3, n is an integer|x = 2k*pi/3 (multiplicity 2), k is an integer|x = 2pi/9 + 2m*pi/3, m is an integer",
    ),
    (r"sin(1*x)/sin(3*x) = 0", r"no solution"),
    (r"cos(1*x)/cos(3*x) = 0", r"no solution"),
    (r"sin(1*x)/cos(3*x) = 0", r"x = k*pi, k is an integer"),
    (
        r"sin(1*x) = sin(3*x)",
        r"x = k*pi, k is an integer|x = pi/4 + n*pi/2, n is an integer",
    ),
    (
        r"cos(1*x) = cos(3*x)",
        r"x = k*pi (multiplicity 2), k is an integer|x = pi/2 + n*pi, n is an integer",
    ),
    (r"(1 - cos(x))/sin(x) = 0", r"no solution"),
    (
        r"sin(x)/(1 - cos(x)) = 0",
        r"x = pi + 2k*pi, k is an integer",
    ),
    (
        r"(1 - cos(x))/x^2 = 0",
        r"x = 2k*pi, k is an integer other than 0",
    ),
    (
        r"(1 - cos(x))/x = 0",
        r"x = 2k*pi, k is an integer other than 0",
    ),
    (
        r"(cos(x) - 1)/x^2 = 0",
        r"x = 2k*pi, k is an integer other than 0",
    ),
    (
        r"sin(x)^2/x = 0",
        r"x = k*pi (multiplicity 2), k is an integer other than 0",
    ),
    (
        r"sin(x)/x = 1",
        r"x ~ -13.899959713976464002205598382182 - 3.352209884853504905194946667497i|x ~ -13.899959713976464002205598382182 + 3.352209884853504905194946667497i|x ~ -7.4976762777763854982723249965899 - 2.7686782829873215324953139130514i|x ~ -7.4976762777763854982723249965899 + 2.7686782829873215324953139130514i|x ~ 7.4976762777763854982723249965899 - 2.7686782829873215324953139130514i|x ~ 7.4976762777763854982723249965899 + 2.7686782829873215324953139130514i|x ~ 13.899959713976464002205598382182 - 3.352209884853504905194946667497i|x ~ 13.899959713976464002205598382182 + 3.352209884853504905194946667497i|note: numeric search: the roots found are listed, there may be others",
    ),
    (r"sin(x)/x^2 = 0", r"x = k*pi, k is an integer other than 0"),
    (
        r"sin(x)/(x*(x - pi)) = 0",
        r"x = k*pi, k is an integer other than 0 and 1",
    ),
    (
        r"sin(pi*x)/(x^2 - 1) = 0",
        r"x = k, k is an integer other than -1 and 1",
    ),
    (
        r"sin(pi*x)/(x*(x - 1)*(x + 2)) = 0",
        r"x = k, k is an integer other than -2, 0 and 1",
    ),
    (
        r"cos(x)/(x - pi/2) = 0",
        r"x = pi/2 + k*pi, k is an integer other than 0",
    ),
    (
        r"cos(x)/(2*x - pi) = 0",
        r"x = pi/2 + k*pi, k is an integer other than 0",
    ),
    (
        r"(exp(x) - 1)/x = 0",
        r"x = 2i*k*pi, k is an integer other than 0",
    ),
    (
        r"(exp(2*x) - 1)/(exp(x) - 1) = 0",
        r"x = i*pi + 2i*k*pi, k is an integer",
    ),
    (r"(exp(x) - 1)/(exp(2*x) - 1) = 0", r"no solution"),
    (r"exp(x) = 1", r"x = 2i*k*pi, k is an integer"),
    (r"exp(x) = -1", r"x = i*pi + 2i*k*pi, k is an integer"),
    (r"exp(x) = i", r"x = i*pi/2 + 2i*k*pi, k is an integer"),
    (r"exp(x) = 0", r"no solution"),
    (
        r"exp(2*x) - 3*exp(x) + 2 = 0",
        r"x = 2i*n*pi, n is an integer|x = ln(2) + 2i*k*pi, k is an integer",
    ),
    (r"exp(x) + exp(-x) = 2", r"x = 2i*k*pi, k is an integer"),
    (
        r"exp(x) - exp(-x) = 2",
        r"x = ln(1 + sqrt(2)) + 2i*n*pi, n is an integer|x = ln(3 - 2sqrt(2))/2 + i*pi + 2i*k*pi, k is an integer",
    ),
    (r"exp(x)*x = exp(x)", r"x = 1"),
    (r"x*exp(x) - exp(x) = 0", r"x = 1"),
    (r"x^2*exp(x) = exp(x)", r"x = -1|x = 1"),
    (
        r"exp(x)*sin(x) = exp(x)",
        r"x = pi/2 + 2k*pi, k is an integer",
    ),
    (r"exp(x)*sin(x) = 0", r"x = k*pi, k is an integer"),
    (r"ln(x) = 0", r"x = 1"),
    (r"ln(x) = 1", r"x = e|  ~ 2.7182818284590452353602874713527"),
    (r"ln(x) = i*pi", r"x = -1"),
    (r"ln(x)/(x - 1) = 0", r"no solution"),
    (
        r"ln(x)/(x - 1) = 1",
        r"no solution|note: no exact method: simple numbers were tried and a numeric search was made, solutions may have been missed",
    ),
    (r"x*ln(x) = 0", r"x = 1"),
    (r"x*ln(x) = ln(x)", r"x = 1 (multiplicity 2)"),
    (r"x^3*ln(x) = x*ln(x)", r"x = -1|x = 1 (multiplicity 2)"),
    (
        r"ln(x)^2 = 1",
        r"x = 1/e|  ~ 0.36787944117144232159552377016146|x = e|  ~ 2.7182818284590452353602874713527",
    ),
    (r"ln(x)*(x - 2) = 0", r"x = 1|x = 2"),
    (r"ln(x^2) = 0", r"x = -1|x = 1"),
    (r"sqrt(x) = 2", r"x = 4"),
    (r"sqrt(x) = -2", r"no solution"),
    (r"sqrt(x) = i", r"x = -1"),
    (r"x*sqrt(x) = sqrt(x)", r"x = 0|x = 1 (multiplicity 2)"),
    (r"sqrt(x)/x = 0", r"no solution"),
    (r"sqrt(x)/x = 1", r"x = 1"),
    (r"(sqrt(x) - 1)/(x - 1) = 1/2", r"no solution"),
    (r"(sqrt(x) - 1)/(x - 1) = 1/3", r"x = 4"),
    (r"(x - 1)/sqrt(x - 1) = 0", r"x = 1"),
    (r"sqrt(x + 1) = x - 1", r"x = 3"),
    (r"sqrt(x^2 - 1) = 0", r"x = -1|x = 1"),
    (r"x^2 = 1", r"x = -1|x = 1"),
    (r"x^2 = -1", r"x = -i|x = i"),
    (
        r"x^3 = 8",
        r"x = 2|x = -1 - i*sqrt(3)|  ~ -1 - 1.7320508075688772935274463415059i|x = -1 + i*sqrt(3)|  ~ -1 + 1.7320508075688772935274463415059i",
    ),
    (
        r"x^3 = i",
        r"x = -sqrt(3)/2 + i/2|  ~ -0.86602540378443864676372317075294 + 0.5i|x = -i|x = sqrt(3)/2 + i/2|  ~ 0.86602540378443864676372317075294 + 0.5i",
    ),
    (
        r"x^4 + 1 = 0",
        r"x = -sqrt(2)/2 - i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = -sqrt(2)/2 + i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i|x = sqrt(2)/2 - i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = sqrt(2)/2 + i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i",
    ),
    (
        r"x^5 + x = i",
        r"x = -sqrt(3)/2 + i/2|  ~ -0.86602540378443864676372317075294 + 0.5i|x = -sqrt(-1/3 + (11/54 + sqrt(69)/18)^(1/3)/2 - (sqrt(69)/18 - 11/54)^(1/3)/2 + i*sqrt(3)*((11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))/2)|  ~ -0.74486176661974423659317042860439 - 0.87743883312334638002475444817926i|x = sqrt(-1/3 - (11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))|  ~ 0.75487766624669276004950889635853i|x = sqrt(-1/3 + (11/54 + sqrt(69)/18)^(1/3)/2 - (sqrt(69)/18 - 11/54)^(1/3)/2 - i*sqrt(3)*((11/54 + sqrt(69)/18)^(1/3) + (sqrt(69)/18 - 11/54)^(1/3))/2)|  ~ 0.74486176661974423659317042860439 - 0.87743883312334638002475444817926i|x = sqrt(3)/2 + i/2|  ~ 0.86602540378443864676372317075294 + 0.5i",
    ),
    (
        r"x^3 + i*x + 1 = 0",
        r"x ~ -1.0046988220954716172011297075992 + 0.34537930092467870057699684146092i|x ~ 0.22454268478871659519199431499714 + 0.69031453455900798607336678217084i|x ~ 0.78015613730675502200913539260203 - 1.0356938354836866866503636236318i",
    ),
    (r"x^2/x = 0", r"x = 0"),
    (r"x^2/x = 1", r"x = 1"),
    (r"x/x = 1", r"true"),
    (r"(x^2 - 1)/(x - 1) = 0", r"x = -1"),
    (r"(x^2 - 1)/(x - 1) = 2", r"x = 1"),
    (r"(x^2 - 4)/(x - 2) = 4", r"x = 2"),
    (r"(x^2 - 4)/(x - 2) = 5", r"x = 3"),
    (r"x*(x - 1)/(x - 1) = 0", r"x = 0"),
    (r"1/x = 0", r"no solution"),
    (r"1/x = 2", r"x = 1/2|  ~ 0.5"),
    (r"1/(x - 1) = 1/(x^2 - 1)", r"x = 0"),
    (r"x/(x^2 - x) = 1/(x - 1)", r"every x is a solution"),
    (r"1/x + 1/(x - 1) = 0", r"x = 1/2|  ~ 0.5"),
    (r"(x - 1)*(x - 2)/((x - 1)*(x - 3)) = 0", r"x = 2"),
    (r"(x^3 - x)/(x^2 - 1) = 0", r"x = 0"),
    (r"(x^3 - x)/(x^2 - 1) = 1", r"x = 1"),
    (r"1/(x^2 + 1) = 1", r"x = 0 (multiplicity 2)"),
    (r"1/(x^2 + 1) = 0", r"no solution"),
    (r"x + 1/x = 2", r"x = 1 (multiplicity 2)"),
    (r"x + 1/x = 0", r"x = -i|x = i"),
    (r"x - 1/x = 0", r"x = -1|x = 1"),
    (r"(x^2 + 1)/x = 0", r"x = -i|x = i"),
    (r"x^2*(x - 1)/x = 0", r"x = 0|x = 1"),
    (r"tan(x)*cos(x) = 1", r"no solution"),
    (r"tan(x)*cos(x)^2 = 0", r"x = n*pi, n is an integer"),
    (
        r"tan(x)^2*cos(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (
        r"cot(x)*sin(x) = 1/2",
        r"x = -pi/3 + 2m*pi, m is an integer|x = pi/3 + 2l*pi, l is an integer",
    ),
    (r"cot(x)*sin(x)^2 = 0", r"x = pi/2 + n*pi, n is an integer"),
    (
        r"cot(x)^2*sin(x) = 0",
        r"x = pi/2 + n*pi (multiplicity 2), n is an integer",
    ),
    (r"tan(x)*cot(x) = 1", r"true"),
    (r"sec(x)*cos(x) = 1", r"true"),
    (
        r"cot(x) + 1/tan(x) = 0",
        r"x = pi/2 + k*pi, k is an integer",
    ),
    (r"cot(x) + tan(x) = 2", r"x = pi/4 + k*pi, k is an integer"),
    (
        r"cot(x) = tan(x)",
        r"x = -pi/4 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (
        r"cot(x)^2 - 1 = 0",
        r"x = -pi/4 + n*pi, n is an integer|x = pi/4 + k*pi, k is an integer",
    ),
    (r"2*cot(x) + 1 = 0", r"x = -atan(2) + k*pi, k is an integer"),
    (r"cot(2*x) = 0", r"x = pi/4 + n*pi/2, n is an integer"),
    (
        r"cot(x)^2 = 0",
        r"x = pi/2 + m*pi (multiplicity 2), m is an integer",
    ),
    (r"x^2*cot(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (r"x/tan(x) = 0", r"x = pi/2 + k*pi, k is an integer"),
    (
        r"x/tanh(x) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (
        r"sinh(x)*cosh(x) = 0",
        r"x = -i*pi/2 + 2i*n*pi, n is an integer|x = 2i*m*pi, m is an integer|x = i*pi + 2i*l*pi, l is an integer|x = i*pi/2 + 2i*k*pi, k is an integer",
    ),
    (r"sin(x)*cos(x) = sin(x)", r"x = k*pi, k is an integer"),
    (r"sin(x)*cos(x) = 1/2", r"x = pi/4 + k*pi, k is an integer"),
    (
        r"sin(x)^3 = sin(x)",
        r"x = -pi/2 + 2m*pi, m is an integer|x = k*pi, k is an integer|x = pi/2 + 2n*pi, n is an integer",
    ),
    (
        r"sin(x)^2 = 1/4",
        r"x = -pi/6 + 2m*pi, m is an integer|x = 5pi/6 + 2n*pi, n is an integer|x = 7pi/6 + 2l*pi, l is an integer|x = pi/6 + 2k*pi, k is an integer",
    ),
    (
        r"cos(x)^2 = 1/4",
        r"x = -2pi/3 + 2l*pi, l is an integer|x = -pi/3 + 2n*pi, n is an integer|x = 2pi/3 + 2m*pi, m is an integer|x = pi/3 + 2k*pi, k is an integer",
    ),
    (
        r"sin(x) + cos(x) = 1",
        r"x = 2k*pi, k is an integer|x = pi/2 + 2n*pi, n is an integer",
    ),
    (r"sin(x) + cos(x) = 0", r"x = -pi/4 + k*pi, k is an integer"),
    (r"sin(x) - cos(x) = 0", r"x = pi/4 + k*pi, k is an integer"),
    (r"sin(x) = cos(x)", r"x = pi/4 + k*pi, k is an integer"),
    (
        r"sin(2*x) = sin(x)",
        r"x = -pi/3 + 2m*pi, m is an integer|x = 2k*pi, k is an integer|x = pi + 2n*pi, n is an integer|x = pi/3 + 2l*pi, l is an integer",
    ),
    (
        r"sin(2*x) = cos(x)",
        r"x = 5pi/6 + 2k*pi, k is an integer|x = pi/2 + m*pi, m is an integer|x = pi/6 + 2n*pi, n is an integer",
    ),
    (
        r"cos(2*x) = cos(x)",
        r"x = -2pi/3 + 2n*pi, n is an integer|x = 2k*pi (multiplicity 2), k is an integer|x = 2pi/3 + 2m*pi, m is an integer",
    ),
    (
        r"cos(2*x) = sin(x)",
        r"x = -pi/2 + 2k*pi, k is an integer|x = 5pi/6 + 2n*pi, n is an integer|x = pi/6 + 2m*pi, m is an integer",
    ),
    (
        r"sin(x)*sin(2*x)*sin(4*x) = 0",
        r"x = m*pi/4, m is an integer",
    ),
    (r"cos(x)*sin(2*x) = 0", r"x = n*pi/2, n is an integer"),
    (r"x*sin(x) = sin(x)", r"x = 1|x = k*pi, k is an integer"),
    (
        r"x^2*sin(x) = 4*sin(x)",
        r"x = -2|x = 2|x = k*pi, k is an integer",
    ),
    (
        r"x^3*sin(x) - x*sin(x) = 0",
        r"x = -1|x = 1|x = k*pi, k is an integer",
    ),
    (
        r"sin(1/x) = 0",
        r"x = 1/(k*pi), k is an integer other than 0",
    ),
    (
        r"x*sin(1/x) = 0",
        r"x = 1/(k*pi), k is an integer other than 0",
    ),
    (r"cos(1/x) = 0", r"x = 2/(pi + 2k*pi), k is an integer"),
    (
        r"sin(x^2) = 0",
        r"x = -sqrt(k)*sqrt(pi), k is an integer|x = sqrt(k)*sqrt(pi), k is an integer",
    ),
    (r"sin(sqrt(x)) = 0", r"x = k^2*pi^2, k is an integer"),
    (
        r"exp(1/x) = 1",
        r"x = -i/(2k*pi), k is an integer other than 0",
    ),
    (r"exp(1/x) = 0", r"no solution"),
    (r"exp(-1/x^2) = 0", r"no solution"),
    (r"x^x = 1", r"x = 1"),
    (r"x^x = 4", r"x = 2"),
    (r"2^x = 8", r"x = 3 + 2i*k*pi/ln(2), k is an integer"),
    (
        r"2^x = -8",
        r"x = 3 + i*pi/ln(2) + 2i*k*pi/ln(2), k is an integer",
    ),
    (r"2^x*x = 2^x", r"x = 1"),
    (r"x*2^x = 0", r"x = 0"),
    (r"x^(1/2) = 3", r"x = 9"),
    (r"x^(1/3) = 2", r"x = 8"),
    (r"x^(1/3) = -2", r"no solution"),
    (r"x^(2/3) = 4", r"x = 8"),
    (r"x^(-2) = 4", r"x = -1/2|  ~ -0.5|x = 1/2|  ~ 0.5"),
    (r"x^(-1/2) = 2", r"x = 1/4|  ~ 0.25"),
    (
        r"atan(x) = 1",
        r"x = tan(1)|  ~ 1.5574077246549022305069748074584",
    ),
    (r"atan(x) = 0", r"x = 0"),
    (r"x*atan(x) = atan(x)", r"x = 0|x = 1"),
    (r"asin(x) = pi/2", r"x = 1"),
    (r"asin(x) = 0", r"x = 0"),
    (r"acos(x) = 0", r"x = 1"),
    (r"acos(x) = pi", r"x = -1"),
    (r"sin(x)/(x^2 + 1) = 0", r"x = k*pi, k is an integer"),
    (r"(x^2 + 1)/sin(x) = 0", r"x = -i|x = i"),
    (r"(x - 1)/sin(pi*x) = 0", r"no solution"),
    (r"(x^2 - 1)/sin(pi*x) = 0", r"no solution"),
    (r"x/sin(x) = 0", r"no solution"),
    (r"x^2/sin(x) = 0", r"no solution"),
    (r"x^2/sin(x)^2 = 0", r"no solution"),
    (
        r"sin(x)^2/(1 - cos(x)) = 0",
        r"x = pi + 2k*pi (multiplicity 2), k is an integer",
    ),
    (
        r"sin(x)^2/(1 + cos(x)) = 0",
        r"x = 2k*pi (multiplicity 2), k is an integer",
    ),
    (r"(1 + cos(x))/sin(x)^2 = 0", r"no solution"),
    (
        r"sin(x)/sin(x/7) = 0",
        r"x = 2pi + 7n*pi, n is an integer|x = 3pi + 7m*pi, m is an integer|x = 4pi + 7l*pi, l is an integer|x = 5pi + 7p*pi, p is an integer|x = 6pi + 7pi*q, q is an integer|x = pi + 7k*pi, k is an integer",
    ),
    (
        r"sin(3*x)/sin(x) = 0",
        r"x = 2pi/3 + n*pi, n is an integer|x = pi/3 + k*pi, k is an integer",
    ),
    (r"tan(x)/sin(2*x) = 0", r"no solution"),
    (
        r"tan(3*x)/tan(x) = 0",
        r"x = 2pi/3 + m*pi, m is an integer|x = pi/3 + k*pi, k is an integer",
    ),
    (
        r"tan(x)/tan(3*x) = 0",
        r"x = 5pi/6 + m*pi, m is an integer|x = pi/6 + n*pi, n is an integer",
    ),
    (
        r"sin(x)*(x - pi)/(x - pi) = 0",
        r"x = k*pi, k is an integer",
    ),
    (
        r"gamma(x) = 1",
        r"x = 1|x = 2|x ~ -1.626659924898260084690988913276 + 0.46921805381744640362400753067267i|note: numeric search: the roots found are listed, there may be others",
    ),
    (r"gamma(x) = 0", r"no solution"),
    (
        r"x/gamma(x) = 0",
        r"x = 0|note: 1/gamma(x) is zero at the poles of gamma(x), a row of whole numbers without end: these solutions are not listed",
    ),
];

const POLYNOMIALS_WITH_COMPLEX_COEFFICIENTS: &[(&str, &str)] = &[
    (
        r"(4 + -1*i)*x^4 + (-4 + 0*i)*x^3 + (3 + 1*i)*x^2 + (-4 + 0*i)*x^1 + (-4 + -2*i)*x^0 = 0",
        r"x ~ -0.56443417180392696446276924789948 - 0.1274490591279337233499998843389i|x ~ -0.13572680774360114920835818879845 + 1.2574344690098381744926478042865i|x ~ 0.28587689054798654432702065954139 - 1.0482006557203111365334964819097i|x ~ 1.3554605595877768634617538359801 + 0.15350936348546550892026032666793i",
    ),
    (
        r"expand((x - (-4 + 3*i))^1*(x - (4 + 1*i))^1*(x - (0 + 4*i))^1) = 0",
        r"x = -4 + 3i|x = 4i|x = 4 + i",
    ),
    (
        r"(x - (3 + -4*i))^2*(x - (-2 + 1*i))^1 = 0",
        r"x = -2 + i|x = 3 - 4i (multiplicity 2)",
    ),
    (
        r"expand((x - (-2 + 0*i))^2) = 0",
        r"x = -2 (multiplicity 2)",
    ),
    (
        r"(-1 + 0*i)*x^2 + (-3 + 2*i)*x^1 + (3 + -1*i)*x^0 = 0",
        r"x = -3/2 - sqrt(17/2 + sqrt(545)/2)/2 + i + i*sqrt(sqrt(545)/2 - 17/2)/2|  ~ -3.7456968589910322097850648644592 + 1.8905921527176106913692036325896i|x = -3/2 + sqrt(17/2 + sqrt(545)/2)/2 + i - i*sqrt(sqrt(545)/2 - 17/2)/2|  ~ 0.74569685899103220978506486445918 + 0.10940784728238930863079636741038i",
    ),
    (
        r"expand((x - (-4 + -1*i))^1*(x - (1 + -4*i))^1*(x - (-3 + -3*i))^1) = 0",
        r"x = -4 - i|x = -3 - 3i|x = 1 - 4i",
    ),
    (
        r"(x - (-4 + 3*i))^2*(x - (0 + -3*i))^1*(x - (-1 + -2*i))^2*(x - (3 + 3*i))^2 = 0",
        r"x = -4 + 3i (multiplicity 2)|x = -1 - 2i (multiplicity 2)|x = -3i|x = 3 + 3i (multiplicity 2)",
    ),
    (
        r"expand((x - (-4 + 0*i))^1*(x - (1 + 0*i))^2*(x - (-3 + 0*i))^1*(x - (3 + 0*i))^2) = 0",
        r"x = -4|x = -3|x = 1 (multiplicity 2)|x = 3 (multiplicity 2)",
    ),
    (
        r"(-4 + 4*i)*x^5 + (2 + 4*i)*x^4 + (3 + 2*i)*x^3 + (0 + 3*i)*x^2 + (3 + 3*i)*x^1 + (0 + 1*i)*x^0 = 0",
        r"x ~ -0.98498472193878870799136203162557 - 0.057565086749760250602265497824413i|x ~ -0.20057429732429850438132530983596 + 0.83783725873201901667937659665795i|x ~ -0.14088572390868030835717426942935 - 0.17810407863906009286407506942782i|x ~ 0.259225960874781763601788033097 - 0.72213214975922408523101108225861i|x ~ 0.81721878229698575712807357779388 + 0.86996405641602541201797505285289i",
    ),
    (
        r"expand((x - (4 + -1*i))^1*(x - (-1 + 2*i))^1*(x - (1 + 4*i))^1*(x - (1 + -4*i))^1) = 0",
        r"x = -1 + 2i|x = 1 - 4i|x = 1 + 4i|x = 4 - i",
    ),
    (r"(x - (-4 + -2*i))^2 = 0", r"x = -4 - 2i (multiplicity 2)"),
    (r"expand((x - (2 + 0*i))^1) = 0", r"x = 2"),
    (
        r"(4 + 3*i)*x^2 + (1 + 2*i)*x^1 + (-4 + -1*i)*x^0 = 0",
        r"x = (1/50)(-10 - 4sqrt(49 + 68i) - 5i + 3i*sqrt(49 + 68i))|  ~ -1.102261805329254338823937010876 + 0.055165606227685822202445931480775i|x = (-1/50)(10 - 4sqrt(49 + 68i) + 5i + 3i*sqrt(49 + 68i))|  ~ 0.70226180532925433882393701087597 - 0.25516560622768582220244593148078i",
    ),
    (r"expand((x - (3 + -3*i))^1) = 0", r"x = 3 - 3i"),
    (
        r"(x - (2 + 2*i))^1*(x - (4 + 1*i))^1 = 0",
        r"x = 2 + 2i|x = 4 + i",
    ),
    (r"expand((x - (-3 + 0*i))^1) = 0", r"x = -3"),
    (
        r"(1 + 3*i)*x^1 + (2 + 4*i)*x^0 = 0",
        r"x = -7/5 + i/5|  ~ -1.4 + 0.2i",
    ),
    (r"expand((x - (3 + -1*i))^1) = 0", r"x = 3 - i"),
    (
        r"(x - (-4 + 3*i))^1*(x - (-4 + 0*i))^1 = 0",
        r"x = -4|x = -4 + 3i",
    ),
    (
        r"expand((x - (1 + 0*i))^2*(x - (0 + 0*i))^2*(x - (-4 + 0*i))^2*(x - (1 + 0*i))^2) = 0",
        r"x = -4 (multiplicity 2)|x = 0 (multiplicity 2)|x = 1 (multiplicity 4)",
    ),
    (
        r"(-1 + -1*i)*x^2 + (4 + 3*i)*x^1 + (2 + -1*i)*x^0 = 0",
        r"x = (1/4)(7 - sqrt(19 + 28i) - i + i*sqrt(19 + 28i))|  ~ -0.21592612767140333238495094332298 + 0.3540410080955421649307414999561i|x = (1/4)(7 + sqrt(19/2 + sqrt(1145)/2) + sqrt(sqrt(1145)/2 - 19/2) - i - i*sqrt(19/2 + sqrt(1145)/2) + i*sqrt(sqrt(1145)/2 - 19/2))|  ~ 3.715926127671403332384950943323 - 0.8540410080955421649307414999561i",
    ),
    (r"expand((x - (-1 + 0*i))^1) = 0", r"x = -1"),
    (
        r"(x - (-1 + -1*i))^2*(x - (3 + -1*i))^2*(x - (3 + -1*i))^2 = 0",
        r"x = -1 - i (multiplicity 2)|x = 3 - i (multiplicity 4)",
    ),
    (r"expand((x - (3 + 0*i))^1) = 0", r"x = 3"),
    (
        r"(3 + 3*i)*x^2 + (-2 + -2*i)*x^1 + (1 + -1*i)*x^0 = 0",
        r"x = (1/12)(4 - sqrt(12 + 4sqrt(10)) - sqrt(4sqrt(10) - 12) - i*sqrt(12 + 4sqrt(10)) + i*sqrt(4sqrt(10) - 12))|  ~ -0.14753842481756097338563242214484 - 0.34659275335185684461806287556982i|x = (-1/12)(-4 - sqrt(12 + 4sqrt(10)) - sqrt(4sqrt(10) - 12) - i*sqrt(12 + 4sqrt(10)) + i*sqrt(4sqrt(10) - 12))|  ~ 0.81420509148422764005229908881151 + 0.34659275335185684461806287556982i",
    ),
    (r"expand((x - (0 + 0*i))^1) = 0", r"x = 0"),
    (
        r"(x - (3 + 4*i))^2*(x - (-2 + -4*i))^1*(x - (0 + 2*i))^2*(x - (-1 + 2*i))^2 = 0",
        r"x = -2 - 4i|x = -1 + 2i (multiplicity 2)|x = 2i (multiplicity 2)|x = 3 + 4i (multiplicity 2)",
    ),
    (r"expand((x - (-2 + 0*i))^1) = 0", r"x = -2"),
    (
        r"(-2 + 3*i)*x^5 + (1 + 3*i)*x^4 + (2 + 1*i)*x^3 + (3 + -4*i)*x^2 + (4 + -2*i)*x^1 + (1 + -2*i)*x^0 = 0",
        r"x ~ -0.93666975708809802535582882337734 - 0.45152946805142437113465763585502i|x ~ -0.46122669519250764963571881482217 + 1.2264796927466869232658797676534i|x ~ -0.22278704213567871422888441450279 + 0.39670489334736251957372160026241i|x ~ -0.11122224702028363029531888791753 - 0.80159173618054906491044563843572i|x ~ 1.1934442029750295579772894021583 + 0.32224431044561630089780959868259i",
    ),
    (
        r"expand((x - (0 + -2*i))^1*(x - (3 + 3*i))^1*(x - (-1 + -2*i))^1*(x - (-2 + -2*i))^1) = 0",
        r"x = -2 - 2i|x = -1 - 2i|x = -2i|x = 3 + 3i",
    ),
    (
        r"(x - (1 + 4*i))^1*(x - (3 + 1*i))^2*(x - (4 + 0*i))^2 = 0",
        r"x = 4 (multiplicity 2)|x = 1 + 4i|x = 3 + i (multiplicity 2)",
    ),
    (
        r"expand((x - (2 + 0*i))^1*(x - (-2 + 0*i))^2*(x - (-4 + 0*i))^1*(x - (-1 + 0*i))^1) = 0",
        r"x = -4|x = -2 (multiplicity 2)|x = -1|x = 2",
    ),
    (
        r"(1 + 1*i)*x^6 + (-2 + 4*i)*x^5 + (2 + -1*i)*x^4 + (4 + -2*i)*x^3 + (1 + 3*i)*x^2 + (2 + -2*i)*x^1 + (4 + 3*i)*x^0 = 0",
        r"x ~ -1.1952612609336229763961233886157 - 3.3947721188108418606299583250235i|x ~ -1.1837712685485825354937047471581 + 0.26252350849362280788905899930348i|x ~ -0.43745742282205404648316869718841 - 0.66225141623349875583045497909143i|x ~ 0.12302575391259129233097579694824 + 0.87093305160120376787097573722595i|x ~ 0.5291566973251220554048561136591 - 0.70103939418383056415448274812611i|x ~ 1.1643075010665462106371649223549 + 0.62460636913334460485486131571163i",
    ),
    (r"expand((x - (2 + 4*i))^1) = 0", r"x = 2 + 4i"),
    (
        r"(x - (4 + -1*i))^2*(x - (-2 + -1*i))^1*(x - (1 + 3*i))^1*(x - (-2 + -4*i))^2 = 0",
        r"x = -2 - 4i (multiplicity 2)|x = -2 - i|x = 1 + 3i|x = 4 - i (multiplicity 2)",
    ),
    (
        r"expand((x - (-2 + 0*i))^1*(x - (1 + 0*i))^2*(x - (0 + 0*i))^2) = 0",
        r"x = -2|x = 0 (multiplicity 2)|x = 1 (multiplicity 2)",
    ),
    (
        r"(3 + -1*i)*x^1 + (3 + -4*i)*x^0 = 0",
        r"x = -13/10 + 9i/10|  ~ -1.3 + 0.9i",
    ),
    (
        r"expand((x - (-3 + 0*i))^1*(x - (3 + -2*i))^1*(x - (0 + 4*i))^1*(x - (2 + 0*i))^1) = 0",
        r"x = -3|x = 2|x = 4i|x = 3 - 2i",
    ),
    (
        r"(x - (0 + 1*i))^1*(x - (3 + 3*i))^1*(x - (-3 + -3*i))^1 = 0",
        r"x = -3 - 3i|x = i|x = 3 + 3i",
    ),
    (r"(4 + 0*i)*x^1 + (2 + 0*i)*x^0 = 0", r"x = -1/2|  ~ -0.5"),
    (r"expand((x - (2 + 1*i))^1) = 0", r"x = 2 + i"),
    (
        r"(x - (-3 + -2*i))^1*(x - (-4 + -2*i))^1 = 0",
        r"x = -4 - 2i|x = -3 - 2i",
    ),
    (r"expand((x - (0 + 0*i))^2) = 0", r"x = 0 (multiplicity 2)"),
    (r"(-1 + -1*i)*x^1 + (2 + 2*i)*x^0 = 0", r"x = 2"),
    (
        r"expand((x - (0 + 3*i))^1*(x - (2 + 2*i))^1*(x - (0 + 2*i))^1*(x - (-1 + -2*i))^1) = 0",
        r"x = -1 - 2i|x = 2i|x = 3i|x = 2 + 2i",
    ),
    (
        r"(x - (2 + -2*i))^2*(x - (-2 + -1*i))^2 = 0",
        r"x = -2 - i (multiplicity 2)|x = 2 - 2i (multiplicity 2)",
    ),
    (
        r"expand((x - (0 + 0*i))^1*(x - (-1 + 0*i))^2*(x - (-1 + 0*i))^1*(x - (1 + 0*i))^2) = 0",
        r"x = -1 (multiplicity 3)|x = 0|x = 1 (multiplicity 2)",
    ),
    (
        r"(-2 + 3*i)*x^5 + (2 + 3*i)*x^4 + (-4 + -2*i)*x^3 + (-2 + -1*i)*x^2 + (1 + 2*i)*x^1 + (-1 + -1*i)*x^0 = 0",
        r"x ~ -0.86580623071994700042673757090398 + 1.5328907043513736527594026845538i|x ~ -0.82448421249741162958017315893383 - 0.15813363425643250225680485753732i|x ~ 0.2829270850318244351639143324313 + 0.51335540368475636269904670234621i|x ~ 0.31097363895830319749263843831596 - 0.38087311735388842768255312190474i|x ~ 0.71177433461184638196574257447516 - 0.58416243334888600859601448438105i",
    ),
    (
        r"expand((x - (0 + 0*i))^1*(x - (1 + 2*i))^1) = 0",
        r"x = 0|x = 1 + 2i",
    ),
    (
        r"(x - (-1 + 1*i))^1*(x - (3 + 4*i))^1*(x - (1 + 3*i))^1 = 0",
        r"x = -1 + i|x = 1 + 3i|x = 3 + 4i",
    ),
    (
        r"expand((x - (4 + 0*i))^2*(x - (4 + 0*i))^2) = 0",
        r"x = 4 (multiplicity 4)",
    ),
    (
        r"(1 + -1*i)*x^5 + (-2 + -1*i)*x^4 + (0 + 3*i)*x^3 + (-3 + 4*i)*x^2 + (2 + 4*i)*x^1 + (2 + 3*i)*x^0 = 0",
        r"x ~ -0.9428893730593186751976433866857 + 1.6999182617083222453843177705149i|x ~ -0.61308546258846466976177921096419 - 0.16897978407077286392064492918167i|x ~ -0.31126371508302193814640257605047 - 1.0084713386065224202084292410583i|x ~ 0.29502972758622789168728596300966 + 0.89485462880392591972239885823241i|x ~ 2.0722088231445773914185392106907 + 0.082678232165047119022357541492589i",
    ),
    (r"expand((x - (1 + -3*i))^1) = 0", r"x = 1 - 3i"),
    (r"(x - (0 + 1*i))^1 = 0", r"x = i"),
    (
        r"expand((x - (-4 + 0*i))^1*(x - (3 + 0*i))^1*(x - (-4 + 0*i))^1*(x - (-2 + 0*i))^2) = 0",
        r"x = -4 (multiplicity 2)|x = -2 (multiplicity 2)|x = 3",
    ),
    (r"(0 + 2*i)*x^1 + (4 + -4*i)*x^0 = 0", r"x = 2 + 2i"),
    (
        r"expand((x - (4 + 2*i))^1*(x - (0 + -2*i))^1*(x - (2 + -2*i))^1*(x - (1 + -4*i))^1) = 0",
        r"x = -2i|x = 1 - 4i|x = 2 - 2i|x = 4 + 2i",
    ),
    (r"(x - (1 + -1*i))^1 = 0", r"x = 1 - i"),
    (
        r"(-4 + -1*i)*x^1 + (4 + 0*i)*x^0 = 0",
        r"x = 16/17 - 4i/17|  ~ 0.94117647058823529411764705882353 - 0.23529411764705882352941176470588i",
    ),
    (r"(x - (2 + -4*i))^1 = 0", r"x = 2 - 4i"),
    (
        r"expand((x - (0 + 0*i))^1*(x - (3 + 0*i))^1*(x - (1 + 0*i))^2*(x - (-1 + 0*i))^1) = 0",
        r"x = -1|x = 0|x = 1 (multiplicity 2)|x = 3",
    ),
    (
        r"(3 + -3*i)*x^6 + (3 + 2*i)*x^5 + (1 + -3*i)*x^4 + (1 + -4*i)*x^3 + (-4 + 2*i)*x^2 + (-2 + 4*i)*x^1 + (1 + -1*i)*x^0 = 0",
        r"x ~ -0.92633988169933354287422803060808 - 0.50134544444050166120432109416498i|x ~ -0.70516946627328658626548817095352 + 0.31312880968834460898025418982697i|x ~ 0.056941501008528862920957207855145 - 1.6988513666973817857574888505415i|x ~ 0.27732704049792163994211607189886 + 0.043976361363876278999586126798313i|x ~ 0.29924386097652465969764099168675 + 0.98910347314193102031707686855373i|x ~ 0.83133027882297829991233526345418 + 0.020654833610398205331559426194092i",
    ),
    (
        r"expand((x - (2 + 0*i))^1*(x - (-2 + 2*i))^1*(x - (-2 + 4*i))^1*(x - (2 + -4*i))^1) = 0",
        r"x = 2|x = -2 + 2i|x = -2 + 4i|x = 2 - 4i",
    ),
    (
        r"(x - (-3 + 4*i))^2*(x - (2 + 0*i))^2*(x - (3 + -3*i))^2*(x - (-3 + -4*i))^1 = 0",
        r"x = 2 (multiplicity 2)|x = -3 - 4i|x = -3 + 4i (multiplicity 2)|x = 3 - 3i (multiplicity 2)",
    ),
    (
        r"expand((x - (-3 + 0*i))^1*(x - (-2 + 0*i))^2*(x - (-3 + 0*i))^2*(x - (1 + 0*i))^2) = 0",
        r"x = -3 (multiplicity 3)|x = -2 (multiplicity 2)|x = 1 (multiplicity 2)",
    ),
    (
        r"(-1 + 2*i)*x^6 + (0 + 1*i)*x^5 + (-2 + 1*i)*x^4 + (1 + 2*i)*x^3 + (-1 + 4*i)*x^2 + (-1 + -1*i)*x^1 + (0 + -4*i)*x^0 = 0",
        r"x ~ -0.96976041210499352896963929615351 + 1.1562542568086458592091546440629i|x ~ -0.80444996725825277843861946021004 + 0.14993343339429789925684905578081i|x ~ -0.62884091334526127163532942174742 - 0.82945896575726664079216590102468i|x ~ 0.51575859439526658805832299405485 - 1.2586063458783217599192518785789i|x ~ 0.70979642916722063547537054439296 + 1.0925185501353481999026608785347i|x ~ 0.77749626914602035550989463966316 - 0.1106409287027035576572467987748i",
    ),
    (
        r"expand((x - (0 + -3*i))^1*(x - (1 + 2*i))^1) = 0",
        r"x = -3i|x = 1 + 2i",
    ),
    (
        r"(x - (2 + 0*i))^2*(x - (1 + -1*i))^2*(x - (1 + 0*i))^1*(x - (0 + -1*i))^2 = 0",
        r"x = 1|x = 2 (multiplicity 2)|x = -i (multiplicity 2)|x = 1 - i (multiplicity 2)",
    ),
    (
        r"expand((x - (1 + 0*i))^1*(x - (3 + 0*i))^1*(x - (3 + 0*i))^2) = 0",
        r"x = 1|x = 3 (multiplicity 3)",
    ),
    (
        r"(0 + -4*i)*x^6 + (1 + -3*i)*x^5 + (2 + 0*i)*x^4 + (-2 + -1*i)*x^3 + (-3 + -2*i)*x^2 + (0 + -4*i)*x^1 + (4 + -3*i)*x^0 = 0",
        r"x ~ -1.1789837231806541026503381776415 + 0.27981411650495432488422584542413i|x ~ -0.78261379766813232009910272245634 - 0.49189106966068763723637219886025i|x ~ -0.50656845540599993903648990697211 + 0.90831314302306295068674661107612i|x ~ 0.13019699470737935961548790783297 - 1.0736743249400738829730889114575i|x ~ 0.68714040183666835193385779165343 - 0.5500893594023106294974421405548i|x ~ 0.90082857971073865023658510758358 + 0.6775274944750548741359307943723i",
    ),
    (
        r"(x - (-3 + 3*i))^2*(x - (4 + 0*i))^1*(x - (-4 + 2*i))^2*(x - (-3 + 0*i))^2 = 0",
        r"x = -3 (multiplicity 2)|x = 4|x = -4 + 2i (multiplicity 2)|x = -3 + 3i (multiplicity 2)",
    ),
    (
        r"expand((x - (-1 + 0*i))^2*(x - (3 + 0*i))^2) = 0",
        r"x = -1 (multiplicity 2)|x = 3 (multiplicity 2)",
    ),
    (
        r"(4 + 4*i)*x^2 + (1 + -1*i)*x^1 + (-2 + -3*i)*x^0 = 0",
        r"x = (1/16)(-sqrt(8 + sqrt(1585)) - sqrt(sqrt(1585) - 8) + 2i - i*sqrt(8 + sqrt(1585)) + i*sqrt(sqrt(1585) - 8))|  ~ -0.78467777261388463130624917742042 + 0.045349470596315344429469213939014i|x = (-1/16)(-sqrt(8 + sqrt(1585)) - sqrt(sqrt(1585) - 8) - 2i - i*sqrt(8 + sqrt(1585)) + i*sqrt(sqrt(1585) - 8))|  ~ 0.78467777261388463130624917742042 + 0.20465052940368465557053078606099i",
    ),
    (
        r"expand((x - (3 + 2*i))^1*(x - (3 + -1*i))^1) = 0",
        r"x = 3 - i|x = 3 + 2i",
    ),
    (
        r"(x - (2 + -2*i))^2*(x - (0 + -2*i))^2 = 0",
        r"x = -2i (multiplicity 2)|x = 2 - 2i (multiplicity 2)",
    ),
    (
        r"expand((x - (-1 + 0*i))^2) = 0",
        r"x = -1 (multiplicity 2)",
    ),
    (
        r"(-2 + 0*i)*x^1 + (0 + -3*i)*x^0 = 0",
        r"x = -3i/2|  ~ -1.5i",
    ),
    (
        r"expand((x - (-3 + -4*i))^1*(x - (3 + 3*i))^1) = 0",
        r"x = -3 - 4i|x = 3 + 3i",
    ),
    (
        r"(x - (0 + -4*i))^2*(x - (-3 + -4*i))^1*(x - (3 + -1*i))^1*(x - (-4 + -1*i))^2 = 0",
        r"x = -4 - i (multiplicity 2)|x = -3 - 4i|x = -4i (multiplicity 2)|x = 3 - i",
    ),
    (
        r"expand((x - (-3 + 0*i))^1*(x - (-3 + 0*i))^2*(x - (0 + 0*i))^1*(x - (3 + 0*i))^1) = 0",
        r"x = -3 (multiplicity 3)|x = 0|x = 3",
    ),
    (
        r"(-1 + 1*i)*x^2 + (2 + -4*i)*x^1 + (3 + 2*i)*x^0 = 0",
        r"x = (1/4)(6 - sqrt(2sqrt(29) - 4) - sqrt(4 + 2sqrt(29)) - 2i + i*sqrt(2sqrt(29) - 4) - i*sqrt(4 + 2sqrt(29)))|  ~ -0.11130108973575326872733986914165 - 0.81030823673184379043855331200686i|x = (-1/4)(-6 - sqrt(2sqrt(29) - 4) - sqrt(4 + 2sqrt(29)) + 2i + i*sqrt(2sqrt(29) - 4) - i*sqrt(4 + 2sqrt(29)))|  ~ 3.1113010897357532687273398691417 - 0.18969176326815620956144668799314i",
    ),
    (r"expand((x - (3 + 1*i))^1) = 0", r"x = 3 + i"),
    (
        r"(x - (-3 + 1*i))^1*(x - (-3 + 4*i))^1*(x - (-3 + 0*i))^1*(x - (-2 + -4*i))^2 = 0",
        r"x = -3|x = -3 + i|x = -3 + 4i|x = -2 - 4i (multiplicity 2)",
    ),
    (
        r"expand((x - (1 + 0*i))^2*(x - (1 + 0*i))^1*(x - (2 + 0*i))^2*(x - (1 + 0*i))^1) = 0",
        r"x = 1 (multiplicity 4)|x = 2 (multiplicity 2)",
    ),
    (
        r"(-2 + 0*i)*x^2 + (-4 + -4*i)*x^1 + (4 + 3*i)*x^0 = 0",
        r"x = -1 - sqrt(16 + 4sqrt(65))/4 - i - i*sqrt(4sqrt(65) - 16)/4|  ~ -2.7365380609346394090026746902153 - 2.0077521704638682707458122044883i|x = -1 + sqrt(16 + 4sqrt(65))/4 - i + i*sqrt(4sqrt(65) - 16)/4|  ~ 0.73653806093463940900267469021528 + 0.0077521704638682707458122044883479i",
    ),
    (
        r"expand((x - (-2 + 0*i))^1*(x - (-4 + 4*i))^1) = 0",
        r"x = -2|x = -4 + 4i",
    ),
    (r"(x - (1 + -3*i))^2 = 0", r"x = 1 - 3i (multiplicity 2)"),
    (
        r"expand((x - (1 + 0*i))^2*(x - (4 + 0*i))^1*(x - (-2 + 0*i))^1*(x - (2 + 0*i))^1) = 0",
        r"x = -2|x = 1 (multiplicity 2)|x = 2|x = 4",
    ),
    (
        r"(2 + 4*i)*x^4 + (2 + -1*i)*x^3 + (0 + 3*i)*x^2 + (-1 + -1*i)*x^1 + (-2 + 2*i)*x^0 = 0",
        r"x ~ -0.41921374210880188247924258060928 + 0.70841065156199326504725676651372i|x ~ -0.34693215623453040057129440321817 - 0.82904728972455069442134899136817i|x = i|x ~ 0.76614589834333228305053698382745 - 0.37936336183744257062590777514555i",
    ),
    (
        r"expand((x - (-1 + -2*i))^1*(x - (-2 + -3*i))^1*(x - (1 + 2*i))^1) = 0",
        r"x = -2 - 3i|x = -1 - 2i|x = 1 + 2i",
    ),
    (r"(x - (1 + 1*i))^1 = 0", r"x = 1 + i"),
    (
        r"expand((x - (-3 + 0*i))^1*(x - (-2 + 0*i))^2*(x - (-1 + 0*i))^2) = 0",
        r"x = -3|x = -2 (multiplicity 2)|x = -1 (multiplicity 2)",
    ),
    (
        r"(1 + 4*i)*x^2 + (2 + -1*i)*x^1 + (1 + 1*i)*x^0 = 0",
        r"x = (-1/34)(-2 - sqrt(15 - 24i) - 9i + 4i*sqrt(15 - 24i))|  ~ -0.10772737323523054690535248525048 - 0.35856440152969734126986860574881i|x = (1/34)(2 - sqrt(15 - 24i) + 9i + 4i*sqrt(15 - 24i))|  ~ 0.22537443205875995867005836760342 + 0.88797616623557969421104507633705i",
    ),
    (
        r"expand((x - (3 + 3*i))^1*(x - (-1 + -4*i))^1) = 0",
        r"x = -1 - 4i|x = 3 + 3i",
    ),
    (
        r"(x - (-3 + -2*i))^1*(x - (2 + -4*i))^1 = 0",
        r"x = -3 - 2i|x = 2 - 4i",
    ),
    (r"expand((x - (1 + 0*i))^1) = 0", r"x = 1"),
    (r"(1 + 0*i)*x^1 + (2 + 0*i)*x^0 = 0", r"x = -2"),
    (
        r"expand((x - (-2 + 2*i))^1*(x - (0 + -3*i))^1*(x - (-4 + 3*i))^1) = 0",
        r"x = -4 + 3i|x = -2 + 2i|x = -3i",
    ),
    (
        r"(x - (-4 + 3*i))^2*(x - (-4 + -3*i))^1*(x - (2 + 2*i))^2 = 0",
        r"x = -4 - 3i|x = -4 + 3i (multiplicity 2)|x = 2 + 2i (multiplicity 2)",
    ),
    (
        r"expand((x - (-4 + 0*i))^1*(x - (2 + 0*i))^2) = 0",
        r"x = -4|x = 2 (multiplicity 2)",
    ),
    (
        r"(4 + -2*i)*x^2 + (-2 + 4*i)*x^1 + (-2 + -1*i)*x^0 = 0",
        r"x = (1/20)(8 - 2sqrt(14 + 2sqrt(65)) - sqrt(2sqrt(65) - 14) - 6i - i*sqrt(14 + 2sqrt(65)) + 2i*sqrt(2sqrt(65) - 14))|  ~ -0.22173663532677864977538362751342 - 0.42867184504569654032347619232498i|x = (1/20)(8 + 2sqrt(28 - 16i) - 6i + i*sqrt(28 - 16i))|  ~ 1.0217366353267786497753836275134 - 0.17132815495430345967652380767502i",
    ),
    (r"expand((x - (0 + 1*i))^1) = 0", r"x = i"),
    (
        r"(x - (0 + 0*i))^2*(x - (3 + 3*i))^1 = 0",
        r"x = 0 (multiplicity 2)|x = 3 + 3i",
    ),
    (r"expand((x - (-4 + 0*i))^1) = 0", r"x = -4"),
    (
        r"(-2 + -2*i)*x^3 + (1 + -3*i)*x^2 + (-1 + -4*i)*x^1 + (-3 + -4*i)*x^0 = 0",
        r"x ~ -0.96233328964778413311607434351087 - 0.14250874951165976559119235597971i|x ~ 0.16355926365454123311087887865072 + 0.96842089054418554672107344235015i|x ~ 0.29877402599324290000519546486015 - 1.8259121410325257811298810863704i",
    ),
    (
        r"expand((x - (-4 + -4*i))^1*(x - (1 + -4*i))^1) = 0",
        r"x = -4 - 4i|x = 1 - 4i",
    ),
    (
        r"(x - (-4 + 2*i))^1*(x - (-4 + -3*i))^1*(x - (1 + -2*i))^2 = 0",
        r"x = -4 - 3i|x = -4 + 2i|x = 1 - 2i (multiplicity 2)",
    ),
    (
        r"expand((x - (-2 + 0*i))^2*(x - (2 + 0*i))^1*(x - (3 + 0*i))^1*(x - (0 + 0*i))^2) = 0",
        r"x = -2 (multiplicity 2)|x = 0 (multiplicity 2)|x = 2|x = 3",
    ),
    (
        r"(2 + -4*i)*x^1 + (-1 + 3*i)*x^0 = 0",
        r"x = 7/10 - i/10|  ~ 0.7 - 0.1i",
    ),
    (r"(x - (1 + 0*i))^1 = 0", r"x = 1"),
    (
        r"expand((x - (-4 + 0*i))^2*(x - (1 + 0*i))^2*(x - (3 + 0*i))^1) = 0",
        r"x = -4 (multiplicity 2)|x = 1 (multiplicity 2)|x = 3",
    ),
    (
        r"(-3 + 1*i)*x^6 + (3 + 1*i)*x^5 + (0 + 2*i)*x^4 + (3 + -3*i)*x^3 + (3 + -2*i)*x^2 + (-4 + -3*i)*x^1 + (3 + 3*i)*x^0 = 0",
        r"x ~ -1.0524355784279042803805413977642 - 0.23885751080822293990473684512579i|x ~ -0.30409226854751029015477279128341 - 1.1975538922156443802330880983412i|x ~ -0.23111604111864182046483296401923 + 0.9493073257324497389633505882344i|x ~ 0.46507057066653189744581215570395 + 1.1281154561027012173244408555834i|x ~ 0.54719179420250501114561290875051 - 0.26033179927650793247965710948386i|x ~ 1.3753815232250194824087220886124 + 0.219320420465224296329690609133i",
    ),
    (
        r"expand((x - (-2 + -2*i))^1*(x - (-3 + -2*i))^1*(x - (-4 + 2*i))^1*(x - (-2 + 4*i))^1) = 0",
        r"x = -4 + 2i|x = -3 - 2i|x = -2 - 2i|x = -2 + 4i",
    ),
    (
        r"(x - (-1 + 1*i))^2*(x - (-2 + -2*i))^1*(x - (0 + 2*i))^1*(x - (-1 + 0*i))^1 = 0",
        r"x = -1|x = -2 - 2i|x = -1 + i (multiplicity 2)|x = 2i",
    ),
    (r"expand((x - (4 + 0*i))^1) = 0", r"x = 4"),
];

const SOLUTIONS_AND_ASSUMPTIONS: &[(&str, &str)] = &[
    (
        r"sin(x) = x",
        r"x = 0 (multiplicity 3)|x ~ -13.899959713976464002205598382182 - 3.352209884853504905194946667497i|x ~ -13.899959713976464002205598382182 + 3.352209884853504905194946667497i|x ~ -7.4976762777763854982723249965899 - 2.7686782829873215324953139130514i|x ~ -7.4976762777763854982723249965899 + 2.7686782829873215324953139130514i|x ~ 7.4976762777763854982723249965899 - 2.7686782829873215324953139130514i|x ~ 7.4976762777763854982723249965899 + 2.7686782829873215324953139130514i|x ~ 13.899959713976464002205598382182 - 3.352209884853504905194946667497i|x ~ 13.899959713976464002205598382182 + 3.352209884853504905194946667497i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"tan(x) = x",
        r"x ~ -14.066193912831473479978965600602|x ~ -10.904121659428899827148702790189|x ~ -7.725251836937707164195068933063|x ~ -4.4934094579090641753078809272803|x = 0 (multiplicity 3)|x ~ 4.4934094579090641753078809272803|x ~ 7.725251836937707164195068933063|x ~ 10.904121659428899827148702790189|x ~ 14.066193912831473479978965600602|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"sinh(x) = x",
        r"x = 0 (multiplicity 3)|x ~ -3.352209884853504905194946667497 - 13.899959713976464002205598382182i|x ~ -3.352209884853504905194946667497 + 13.899959713976464002205598382182i|x ~ -2.7686782829873215324953139130514 - 7.4976762777763854982723249965899i|x ~ -2.7686782829873215324953139130514 + 7.4976762777763854982723249965899i|x ~ 2.7686782829873215324953139130514 - 7.4976762777763854982723249965899i|x ~ 2.7686782829873215324953139130514 + 7.4976762777763854982723249965899i|x ~ 3.352209884853504905194946667497 - 13.899959713976464002205598382182i|x ~ 3.352209884853504905194946667497 + 13.899959713976464002205598382182i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"e^x = 1 + x + x^2/2",
        r"x = 0 (multiplicity 3)|x ~ 3.8386020477829022514350687206008 - 8.3668155066743487732850519090823i|x ~ 3.8386020477829022514350687206008 + 8.3668155066743487732850519090823i|x ~ 4.8572639597351853931462287582793 - 14.958911406214417429691719740998i|x ~ 4.8572639597351853931462287582793 + 14.958911406214417429691719740998i|x ~ 5.5206265541122946298839458057995 - 21.398462005617317914398008230079i|x ~ 5.5206265541122946298839458057995 + 21.398462005617317914398008230079i|x ~ 6.0161784134914232208096488302173 - 27.778959614418298536803907586339i|x ~ 6.0161784134914232208096488302173 + 27.778959614418298536803907586339i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"x^2 = sin(x)^2",
        r"x = 0 (multiplicity 4)|x ~ -13.899959713976464002205598382182 - 3.352209884853504905194946667497i|x ~ -13.899959713976464002205598382182 + 3.352209884853504905194946667497i|x ~ -10.71253739727926042036234315951 - 3.1031487458252496450311047587207i|x ~ -10.71253739727926042036234315951 + 3.1031487458252496450311047587207i|x ~ -7.4976762777763854982723249965899 - 2.7686782829873215324953139130514i|x ~ -7.4976762777763854982723249965899 + 2.7686782829873215324953139130514i|x ~ -4.2123922304906606009837705615666 - 2.2507286116018605428400150301647i|x ~ -4.2123922304906606009837705615666 + 2.2507286116018605428400150301647i|x ~ 4.2123922304906606009837705615666 - 2.2507286116018605428400150301647i|x ~ 4.2123922304906606009837705615666 + 2.2507286116018605428400150301647i|x ~ 7.4976762777763854982723249965899 - 2.7686782829873215324953139130514i|x ~ 7.4976762777763854982723249965899 + 2.7686782829873215324953139130514i|x ~ 10.71253739727926042036234315951 - 3.1031487458252496450311047587207i|x ~ 10.71253739727926042036234315951 + 3.1031487458252496450311047587207i|x ~ 13.899959713976464002205598382182 - 3.352209884853504905194946667497i|x ~ 13.899959713976464002205598382182 + 3.352209884853504905194946667497i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"sin(x) = 2x",
        r"x = 0|x ~ -13.852187673012671235868507190174 - 4.055605854191625860747518275303i|x ~ -13.852187673012671235868507190174 + 4.055605854191625860747518275303i|x ~ -7.4133784670155258818674256809328 - 3.4890279102440942855031425555139i|x ~ -7.4133784670155258818674256809328 + 3.4890279102440942855031425555139i|x ~ -2.1773189849653067526304242460601i|x ~ 2.1773189849653067526304242460601i|x ~ 7.4133784670155258818674256809328 - 3.4890279102440942855031425555139i|x ~ 7.4133784670155258818674256809328 + 3.4890279102440942855031425555139i|x ~ 13.852187673012671235868507190174 - 4.055605854191625860747518275303i|x ~ 13.852187673012671235868507190174 + 4.055605854191625860747518275303i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"cos(x) = 1 - x^2/2",
        r"x = 0 (multiplicity 4)|x ~ -14.99535255555277099654464999318 - 5.5373565659746430649906278261028i|x ~ -14.99535255555277099654464999318 + 5.5373565659746430649906278261028i|x ~ -8.4247844609813212019675411231331 - 4.5014572232037210856800300603293i|x ~ -8.4247844609813212019675411231331 + 4.5014572232037210856800300603293i|x ~ 8.4247844609813212019675411231331 - 4.5014572232037210856800300603293i|x ~ 8.4247844609813212019675411231331 + 4.5014572232037210856800300603293i|x ~ 14.99535255555277099654464999318 - 5.5373565659746430649906278261028i|x ~ 14.99535255555277099654464999318 + 5.5373565659746430649906278261028i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"x! = 24",
        r"x ~ -2.978740013245249323210436288544|x ~ -0.95924746764792921050858999682467|x = 4|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"x! = 6",
        r"x ~ -4.9929812252785501036135712915411|x ~ -2.9081830467208065234887447176962|x ~ -0.84477781383966688732094138277195|x = 3|note: numeric search: the roots found are listed, there may be others",
    ),
    (r"5! = 120", r"true"),
    (r"3! != 6", r"false"),
    (
        r"gamma(x) = 24",
        r"x ~ -1.978740013245249323210436288544|x ~ 0.040752532352070789491410003175334|x = 5|note: numeric search: the roots found are listed, there may be others",
    ),
    (r"x^x = 27", r"x = 3"),
    (
        r"x^3 = 3^x",
        r"x ~ 2.4780526802883024118937365168947|x = 3|x ~ -0.55292453255886022413452591097896 - 0.6010604334267696109036925025611i|x ~ -0.55292453255886022413452591097896 + 0.6010604334267696109036925025611i|x ~ 6.3944220571042760358313493416278 - 8.2000364721998215546339741858817i|x ~ 6.3944220571042760358313493416278 + 8.2000364721998215546339741858817i|x ~ 7.6203345627082028590070812233326 - 14.398582558585733719423406646056i|x ~ 7.6203345627082028590070812233326 + 14.398582558585733719423406646056i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"2^x = x^2",
        r"x ~ -0.76666469596212309311120442251031|x = 2|x = 4|x ~ 7.6545064969779272738239370272838 - 11.953678912263155851182329972247i|x ~ 7.6545064969779272738239370272838 + 11.953678912263155851182329972247i|x ~ 9.0907298607480423450283502910825 - 21.507950350537752220346927030966i|x ~ 9.0907298607480423450283502910825 + 21.507950350537752220346927030966i|x ~ 10.036848572412826468454086749298 - 30.818063282987779427062095259386i|x ~ 10.036848572412826468454086749298 + 30.818063282987779427062095259386i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"x/gamma(x) = 0",
        r"x = 0|note: 1/gamma(x) is zero at the poles of gamma(x), a row of whole numbers without end: these solutions are not listed",
    ),
    (r"1/gamma(x) = 0", r"error: no method for this equation"),
    (
        r"abs(x)/x = 1",
        r"x ~ 0.01|x ~ 0.03|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"abs(x) = x",
        r"x = 0|x ~ 0.01|x ~ 0.03|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"floor(x) = 2",
        r"x ~ 2.01|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"ceil(x) = 2",
        r"x ~ 1.01|x ~ 1.05|x ~ 1.51|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"sign(x) = 1",
        r"x ~ 0.01|x ~ 0.11|x ~ 0.51|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"frac(x) = 0",
        r"x = -6|x = -5|x = -4|x = -3|x = -2|x = -1|x = 0|x = 1|x = 2|x = 3|x = 4|x = 5|x = 6|x ~ 7|x ~ 10|x ~ 30|x ~ 100|x = -i|x = i|x ~ 5i|x ~ 1 + i|x ~ 2 + 3i|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"max(x, 1) = 1",
        r"x = -1|x = 0|x = 1|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: no exact method: simple numbers were tried and a numeric search was made, solutions may have been missed",
    ),
    (
        r"re(x) = 1",
        r"x = 1|note: the equation holds on whole stretches of numbers: the ones listed are examples",
    ),
    (
        r"conj(x) = x",
        r"x ~ -1.1|x = -1|x ~ -0.9|x = 0|x = 1|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"abs(x - 1) = abs(x + 1)",
        r"x = 0|x = -i|x = i|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: no exact method: simple numbers were tried and a numeric search was made, solutions may have been missed",
    ),
    (
        r"abs(x) + abs(x - 2) = 2",
        r"x = 0|x ~ 0.01|x ~ 0.11|x ~ 0.51|x = 2|note: the equation holds on whole stretches of numbers: the ones listed are examples|note: numeric search: the roots found are listed, there may be others",
    ),
    (r"minimize(abs(x)+abs(x-2))", r"[2, x = 0, x = 2]"),
    (r"(x - i)^2*(x + i) = 0", r"x = -i|x = i (multiplicity 2)"),
    (
        r"(x - (1+i))^3*(x - (1-i))^2 = 0",
        r"x = 1 - i (multiplicity 2)|x = 1 + i (multiplicity 3)",
    ),
    (r"(x-1)^2*(x^2-1) = 0", r"x = -1|x = 1 (multiplicity 3)"),
    (
        r"sin(x)*tan(x) = 0",
        r"x = k*pi (multiplicity 2), k is an integer",
    ),
    (r"x*sin(x) = 0", r"x = k*pi, k is an integer"),
    (r"sin(x) = 0, cos(x) = 1", r"x = 2k*pi, k is an integer"),
    (
        r"sin(x) = 0, cos(x) = -1",
        r"x = pi + 2k*pi, k is an integer",
    ),
    (r"sin(x) = 0, cos(x) = 0", r"no solution"),
    (
        r"sin(x) = 0, x^2 = pi^2",
        r"x = -pi|  ~ -3.1415926535897932384626433832795|x = pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"e^x = 1, x^2 = -4pi^2",
        r"x = -2i*pi|  ~ -6.283185307179586476925286766559i|x = 2i*pi|  ~ 6.283185307179586476925286766559i",
    ),
    (r"sin(x) = 0, sin(x/2) = 0", r"x = 2k*pi, k is an integer"),
    (r"cos(x) = 1, cos(x/3) = 1", r"x = 6k*pi, k is an integer"),
    (
        r"sin(x) = 0, sin(x/3) = 0, cos(x) = 1",
        r"x = 6k*pi, k is an integer",
    ),
    (r"exp(x) = 1, sin(x) = 0", r"x = 0"),
    (
        r"tan(x) = 1, sin(x) = sqrt(2)/2",
        r"x = pi/4 + 2k*pi, k is an integer",
    ),
    (
        r"sin(x) = 1/2, cos(x) = sqrt(3)/2",
        r"x = pi/6 + 2k*pi|k is an integer",
    ),
    (
        r"sin(x)^2 = 1, cos(2x) = -1",
        r"x = -pi/2 + 2k*pi|x = pi/2 + 2k*pi|k is an integer",
    ),
    (r"x^2 = 1, x^3 = 1", r"x = 1"),
    (r"x = 1, x^2 = 2", r"no solution"),
    (
        r"solve([sin(x) = 0, cos(x) = 1], x)",
        r"x = 2k*pi, k is an integer",
    ),
    (r"x^2 = 4; ans", r"x = -2|x = 2|[x = -2, x = 2]"),
    (
        r"x^2 = 2; ans",
        r"x = -sqrt(2)|  ~ -1.4142135623730950488016887242097|x = sqrt(2)|  ~ 1.4142135623730950488016887242097|[x = -sqrt(2), x = sqrt(2)]",
    ),
    (
        r"x^2 = 2; N(ans)",
        r"x = -sqrt(2)|  ~ -1.4142135623730950488016887242097|x = sqrt(2)|  ~ 1.4142135623730950488016887242097|[x = -1.4142135623730950488016887242097, x = 1.4142135623730950488016887242097]",
    ),
    (
        r"x^3 = 1; ans",
        r"x = 1|x = -1/2 - i*sqrt(3)/2|  ~ -0.5 - 0.86602540378443864676372317075294i|x = -1/2 + i*sqrt(3)/2|  ~ -0.5 + 0.86602540378443864676372317075294i|[x = 1, x = -1/2 - i*sqrt(3)/2, x = -1/2 + i*sqrt(3)/2]",
    ),
    (r"2x = 4; ans", r"x = 2|x = 2"),
    (r"sin(x) = 0; ans", r"x = k*pi, k is an integer|x = k*pi"),
    (
        r"x^2 + y^2 = 25, x*y = 12; ans",
        r"x = -3, y = -4|x = -4, y = -3|x = 4, y = 3|x = 3, y = 4|[[x = -3, y = -4], [x = -4, y = -3], [x = 4, y = 3], [x = 3, y = 4]]",
    ),
    (
        r"x + y = 3, x - y = 1; ans",
        r"x = 2, y = 1|[[x = 2, y = 1]]",
    ),
    (r"x^2 = 4; length(ans)", r"x = -2|x = 2|2"),
    (r"x = 1, x = 2", r"no solution"),
    (r"[x = 1, x = 2]", r"no solution"),
    (
        r"s := solve(x^2 = 4, x); s",
        r"x = -2|x = 2|[x = -2, x = 2]",
    ),
    (
        r"s := solve(x^2 = 2, x); N(s)",
        r"x = -sqrt(2)|  ~ -1.4142135623730950488016887242097|x = sqrt(2)|  ~ 1.4142135623730950488016887242097|[x = -1.4142135623730950488016887242097, x = 1.4142135623730950488016887242097]",
    ),
    (r"s := solve(x^2 = 4, x); length(s)", r"x = -2|x = 2|2"),
    (
        r"s := solve([x + y = 3, x - y = 1], [x, y]); s",
        r"x = 2, y = 1|[[x = 2, y = 1]]",
    ),
    (
        r"s := qsolve(x^2 = -1); s",
        r"x = u, u is any unit pure quaternion|note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1|x = u",
    ),
    (r"eq := x^2 = 4; eq", r"x = -2|x = 2|x = -2|x = 2"),
    (
        r"sys := [x + y = 3, x - y = 1]; sys",
        r"x = 2, y = 1|x = 2, y = 1",
    ),
    (
        r"f(a) := solve(x^2 = a, x); f(9)",
        r"f(a) := solve(x^2 = a, x)|x = -3|x = 3",
    ),
    (
        r"f(a) := [x = a, x = 2a]; f(1)",
        r"f(a) := [x = a, x = 2a]|[x = 1, x = 2]",
    ),
    (r"pi := solve(x = 1, x)", r"error: `pi` is a reserved name"),
    (
        r"sin := solve(x = 1, x)",
        r"error: `sin` is a reserved name",
    ),
    (r"1 + solve(x = 1, x)", r"solve(x = 1, x) + 1"),
    (r"assume(x, real); x^2 = -1", r"true|no real solution"),
    (r"assume(x, real); x^4 = 1", r"true|x = -1|x = 1"),
    (r"assume(x, real); x^3 = 8", r"true|x = 2"),
    (r"assume(x, real); sin(x) = 2", r"true|no real solution"),
    (r"assume(x, real); exp(x) = -1", r"true|no real solution"),
    (r"assume(x, real); exp(x) = 1", r"true|x = 0"),
    (
        r"assume(x, real); x^2 + 2x + 5 = 0",
        r"true|no real solution",
    ),
    (r"assume(x, positive); x^2 = 4", r"true|x = 2"),
    (
        r"assume(x, positive); x^2 = -4",
        r"true|no positive solution",
    ),
    (
        r"assume(x, positive); x^3 - 6x^2 + 11x - 6 = 0",
        r"true|x = 1|x = 2|x = 3",
    ),
    (
        r"assume(x, positive); x^2 + 3x + 2 = 0",
        r"true|no positive solution",
    ),
    (
        r"assume(x, positive); sin(x) = 0",
        r"true|x = k*pi, k is an integer",
    ),
    (r"assume(x, negative); x^2 = 4", r"true|x = -2"),
    (r"assume(x, nonnegative); x^2 = 4", r"true|x = 2"),
    (r"assume(x, nonnegative); x^3 = x", r"true|x = 0|x = 1"),
    (r"assume(x, nonzero); x^2 = x", r"true|x = 1"),
    (r"assume(x, integer); 2x = 1", r"true|no integer solution"),
    (r"assume(x, integer); x^2 - 5x + 6 = 0", r"true|x = 2|x = 3"),
    (
        r"assume(x, integer); 6x^2 - 5x + 1 = 0",
        r"true|no integer solution",
    ),
    (r"assume(n, integer); n^2 = 2", r"true|no integer solution"),
    (
        r"assume(n, integer); assume(n, positive); n^2 = 2",
        r"true|true|no positive integer solution",
    ),
    (
        r"assume(n, integer); assume(n, positive); n^2 = 9",
        r"true|true|n = 3",
    ),
    (r"assume(x > 0); x^2 = 4", r"true|x = 2"),
    (
        r"assume(x, real); assume(y, real); x^2 + y^2 = -1, x = y",
        r"true|true|no real solution",
    ),
    (
        r"assume(x, positive); assume(y, real); x^2 = 4, y = x + 1",
        r"true|true|x = 2, y = 3",
    ),
    (
        r"assume(x, positive); assume(y, negative); x^2 = 4, y^2 = 9",
        r"true|true|x = 2, y = -3",
    ),
    (
        r"assume(x, positive); x + y = 1, x - y = 5",
        r"true|x = 3, y = -2",
    ),
    (
        r"assume(x, positive); x + y = 1, x - y = -5",
        r"true|no solution fits what is assumed about the unknowns",
    ),
    (
        r"assume(x, integer); assume(y, integer); 2x + y = 2, x - y = 0",
        r"true|true|no integer solution",
    ),
    (
        r"assume(x, real); x^2 = -1; forget(x); x^2 = -1",
        r"true|no real solution|true|x = -i|x = i",
    ),
    (r"arg(cos(1) + i*sin(1))", r"1"),
    (r"arg(cos(1) - i*sin(1))", r"-1"),
    (
        r"arg(-cos(1) - i*sin(1))",
        r"1 - pi|  ~ -2.1415926535897932384626433832795",
    ),
    (
        r"arg(sin(1) + i*cos(1))",
        r"pi/2 - 1|  ~ 0.57079632679489661923132169163975",
    ),
    (
        r"arg(sin(1) - i*cos(1))",
        r"1 - pi/2|  ~ -0.57079632679489661923132169163975",
    ),
    (
        r"arg(-sin(2) + i*cos(2))",
        r"2 - 3pi/2|  ~ -2.7123889803846898576939650749193",
    ),
    (
        r"arg(2cos(4) + 2i*sin(4))",
        r"4 - 2pi|  ~ -2.283185307179586476925286766559",
    ),
    (
        r"arg(3cos(10) + 3i*sin(10))",
        r"10 - 4pi|  ~ -2.566370614359172953850573533118",
    ),
    (
        r"arg(cos(1/3)*sqrt(2) + i*sqrt(2)*sin(1/3))",
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (
        r"arg(cos(sqrt(2)) + i*sin(sqrt(2)))",
        r"sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (
        r"arg(cos(1) + 2i*sin(1))",
        r"atan(2sin(1)/cos(1))|  ~ 1.2601445524562140924924436558742",
    ),
    (
        r"arg(cos(1) + i*sin(2))",
        r"atan(sin(2)/cos(1))|  ~ 1.0346542555362988770077341059738",
    ),
    (
        r"arg(cos(pi/7) + i*sin(pi/7))",
        r"pi/7|  ~ 0.4487989505128276054946633404685",
    ),
    (r"atan2(sin(1), cos(1))", r"1"),
    (r"atan2(-sin(1), cos(1))", r"-1"),
    (
        r"exp(x) = cos(1) + i*sin(1)",
        r"x = i + 2i*k*pi, k is an integer",
    ),
    (
        r"x^3 = cos(1) + i*sin(1)",
        r"x = (cos(1) + i*sin(1))^(1/3)*(-1 + i*sqrt(3))/2|  ~ -0.7558373925663835534867570928594 + 0.6547593725930546767715833821546i|x = -(cos(1) + i*sin(1))^(1/3)*(1 + i*sqrt(3))/2|  ~ -0.18911955374835411090152691481648 - 0.98195406938920692094492746742222i|x = cos(1/3) + i*sin(1/3)|  ~ 0.94495694631473766438828400767588 + 0.32719469679615224417334408526762i",
    ),
    (r"polar(i)", r"e^(i*pi/2)"),
    (r"polar(-1)", r"e^(i*pi)"),
    (r"polar(-i)", r"e^(-i*pi/2)"),
    (r"polar(2i)", r"2e^(i*pi/2)"),
    (r"polar(1 + i)", r"sqrt(2)*e^(i*pi/4)"),
    (r"polar(0)", r"0"),
    (r"polar(3)", r"3e^0"),
    (r"polar(cos(2) + i*sin(2))", r"e^(2i)"),
    (r"polar(1/2 + i*sqrt(3)/2)", r"e^(i*pi/3)"),
    (r"polar(1 + 2i)", r"sqrt(5)*e^(i*atan(2))"),
    (r"polar(x)", r"abs(x)*e^(i*arg(x))"),
    (r"convert(2 m, cm)^2", r"4 m^2"),
];

const REAL_DOMAIN_ADDITIONS: &[(&str, &str)] = &[
    (r"cos(abs(x))", r"cos(x)"),
    (r"cosh(abs(x))", r"cosh(x)"),
    (r"x*y = 0, x + y = 1", r"x = 0, y = 1|x = 1, y = 0"),
    (
        r"sin(x) + y = 1, x - y = 1",
        r"x = 1.1060601577062719106167372970301, y = 0.10606015770627191061673729703007|note: numeric search: the roots found are listed, there may be others",
    ),
    (
        r"x^5 + x = 1",
        r"x = -1/3 + (25/54 + sqrt(69)/18)^(1/3) + (25/54 - sqrt(69)/18)^(1/3)|  ~ 0.75487766624669276004950889635853",
    ),
];

const UNITS_IN_SI: &[(&str, &str)] = &[
    (r"siunits(1 m)", r"1 m"),
    (r"siunits(1 inch)", r"0.0254 m"),
    (r"siunits(1 ft)", r"0.3048 m"),
    (r"siunits(1 yd)", r"0.9144 m"),
    (r"siunits(1 mi)", r"1609.344 m"),
    (r"siunits(1 nmi)", r"1852 m"),
    (r"siunits(1 au)", r"149597870700 m"),
    (r"siunits(1 ly)", r"9460730472580800 m"),
    (
        r"siunits(1 pc)",
        r"(96939420213600000/pi) m|  ~ 30856775814913672.789139379577965 m",
    ),
    (r"siunits(1 angstrom)", r"0.0000000001 m"),
    (r"siunits(1 micron)", r"0.000001 m"),
    (r"siunits(1 mil)", r"0.0000254 m"),
    (r"siunits(1 fathom)", r"1.8288 m"),
    (r"siunits(1 furlong)", r"201.168 m"),
    (r"siunits(1 g)", r"0.001 kg"),
    (r"siunits(1 t)", r"1000 kg"),
    (r"siunits(1 lb)", r"0.45359237 kg"),
    (r"siunits(1 oz)", r"0.028349523125 kg"),
    (r"siunits(1 st)", r"6.35029318 kg"),
    (r"siunits(1 ton)", r"907.18474 kg"),
    (r"siunits(1 longton)", r"1016.0469088 kg"),
    (r"siunits(1 grain)", r"0.00006479891 kg"),
    (r"siunits(1 ct)", r"0.0002 kg"),
    (
        r"siunits(1 slug)",
        r"(8896443230521/609600000000) kg|  ~ 14.593902937206364829396325459318 kg",
    ),
    (r"siunits(1 u)", r"1.66053906892e-27 kg"),
    (r"siunits(1 s)", r"1 s"),
    (r"siunits(1 min)", r"60 s"),
    (r"siunits(1 h)", r"3600 s"),
    (r"siunits(1 d)", r"86400 s"),
    (r"siunits(1 wk)", r"604800 s"),
    (r"siunits(1 fortnight)", r"1209600 s"),
    (r"siunits(1 yr)", r"31557600 s"),
    (r"siunits(1 A)", r"1 A"),
    (r"siunits(1 K)", r"1 K"),
    (r"siunits(1 degC)", r"274.15 K"),
    (
        r"siunits(1 degF)",
        r"(46067/180) K|  ~ 255.92777777777777777777777777778 K",
    ),
    (
        r"siunits(1 degR)",
        r"(5/9) K|  ~ 0.55555555555555555555555555555556 K",
    ),
    (r"siunits(1 mol)", r"1 mol"),
    (r"siunits(1 cd)", r"1 cd"),
    (r"siunits(1 lm)", r"1 cd"),
    (r"siunits(1 lx)", r"1 cd/m^2"),
    (r"siunits(1 rad)", r"1"),
    (
        r"siunits(1 deg)",
        r"pi/180|  ~ 0.017453292519943295769236907684886",
    ),
    (
        r"siunits(1 arcmin)",
        r"pi/10800|  ~ 0.00029088820866572159615394846141477",
    ),
    (
        r"siunits(1 arcsec)",
        r"pi/648000|  ~ 0.0000048481368110953599358991410235795",
    ),
    (
        r"siunits(1 rev)",
        r"2pi|  ~ 6.283185307179586476925286766559",
    ),
    (
        r"siunits(1 grad)",
        r"pi/200|  ~ 0.015707963267948966192313216916398",
    ),
    (r"siunits(1 sr)", r"1"),
    (r"siunits(1 percent)", r"1/100|  ~ 0.01"),
    (r"siunits(1 ppm)", r"1/1000000|  ~ 0.000001"),
    (r"siunits(1 ppb)", r"1/1000000000|  ~ 1e-9"),
    (r"siunits(1 Hz)", r"1 s^-1"),
    (
        r"siunits(1 rpm)",
        r"(pi/30) s^-1|  ~ 0.10471975511965977461542144610932 s^-1",
    ),
    (r"siunits(1 Bq)", r"1 s^-1"),
    (r"siunits(1 Ci)", r"37000000000 s^-1"),
    (r"siunits(1 mph)", r"0.44704 m/s"),
    (
        r"siunits(1 kph)",
        r"(5/18) m/s|  ~ 0.27777777777777777777777777777778 m/s",
    ),
    (
        r"siunits(1 kn)",
        r"(463/900) m/s|  ~ 0.51444444444444444444444444444444 m/s",
    ),
    (r"siunits(1 c)", r"299792458 m/s"),
    (r"siunits(1 gn)", r"9.80665 m/s^2"),
    (r"siunits(1 N)", r"1 kg*m/s^2"),
    (r"siunits(1 dyn)", r"0.00001 kg*m/s^2"),
    (r"siunits(1 lbf)", r"4.4482216152605 kg*m/s^2"),
    (r"siunits(1 kgf)", r"9.80665 kg*m/s^2"),
    (r"siunits(1 Pa)", r"1 kg/(m*s^2)"),
    (r"siunits(1 bar)", r"100000 kg/(m*s^2)"),
    (r"siunits(1 atm)", r"101325 kg/(m*s^2)"),
    (
        r"siunits(1 torr)",
        r"(20265/152) kg/(m*s^2)|  ~ 133.32236842105263157894736842105 kg/(m*s^2)",
    ),
    (r"siunits(1 mmHg)", r"133.322387415 kg/(m*s^2)"),
    (r"siunits(1 inHg)", r"3386.388640341 kg/(m*s^2)"),
    (
        r"siunits(1 psi)",
        r"(8896443230521/1290320000) kg/(m*s^2)|  ~ 6894.7572931683613367226734453469 kg/(m*s^2)",
    ),
    (r"siunits(1 J)", r"1 kg*m^2/s^2"),
    (r"siunits(1 cal)", r"4.184 kg*m^2/s^2"),
    (r"siunits(1 eV)", r"1.602176634e-19 kg*m^2/s^2"),
    (r"siunits(1 Wh)", r"3600 kg*m^2/s^2"),
    (r"siunits(1 BTU)", r"1055.05585262 kg*m^2/s^2"),
    (r"siunits(1 erg)", r"0.0000001 kg*m^2/s^2"),
    (r"siunits(1 therm)", r"105505585.262 kg*m^2/s^2"),
    (r"siunits(1 ftlbf)", r"1.3558179483314004 kg*m^2/s^2"),
    (r"siunits(1 tonTNT)", r"4184000000 kg*m^2/s^2"),
    (r"siunits(1 W)", r"1 kg*m^2/s^3"),
    (r"siunits(1 hp)", r"745.69987158227022 kg*m^2/s^3"),
    (r"siunits(1 PS)", r"735.49875 kg*m^2/s^3"),
    (r"siunits(1 C)", r"1 A*s"),
    (r"siunits(1 Ah)", r"3600 A*s"),
    (r"siunits(1 V)", r"1 kg*m^2/(A*s^3)"),
    (r"siunits(1 ohm)", r"1 kg*m^2/(A^2*s^3)"),
    (r"siunits(1 F)", r"1 A^2*s^4/(kg*m^2)"),
    (r"siunits(1 S)", r"1 A^2*s^3/(kg*m^2)"),
    (r"siunits(1 H)", r"1 kg*m^2/(A^2*s^2)"),
    (r"siunits(1 Wb)", r"1 kg*m^2/(A*s^2)"),
    (r"siunits(1 T)", r"1 kg/(A*s^2)"),
    (r"siunits(1 gauss)", r"0.0001 kg/(A*s^2)"),
    (r"siunits(1 Mx)", r"0.00000001 kg*m^2/(A*s^2)"),
    (r"siunits(1 l)", r"0.001 m^3"),
    (r"siunits(1 cc)", r"0.000001 m^3"),
    (r"siunits(1 gal)", r"0.003785411784 m^3"),
    (r"siunits(1 qt)", r"0.000946352946 m^3"),
    (r"siunits(1 pt)", r"0.000473176473 m^3"),
    (r"siunits(1 cup)", r"0.0002365882365 m^3"),
    (r"siunits(1 floz)", r"0.0000295735295625 m^3"),
    (r"siunits(1 tbsp)", r"0.00001478676478125 m^3"),
    (r"siunits(1 tsp)", r"0.00000492892159375 m^3"),
    (r"siunits(1 bbl)", r"0.158987294928 m^3"),
    (r"siunits(1 impgal)", r"0.00454609 m^3"),
    (r"siunits(1 ha)", r"10000 m^2"),
    (r"siunits(1 acre)", r"4046.8564224 m^2"),
    (r"siunits(1 barn)", r"1e-28 m^2"),
    (r"siunits(1 bit)", r"1 bit"),
    (r"siunits(1 B)", r"8 bit"),
    (r"siunits(1 bps)", r"1 bit/s"),
    (r"siunits(1 Gy)", r"1 m^2/s^2"),
    (r"siunits(1 Sv)", r"1 m^2/s^2"),
    (r"siunits(1 rem)", r"0.01 m^2/s^2"),
    (r"siunits(1 kat)", r"1 mol/s"),
    (r"siunits(1 M)", r"1000 mol/m^3"),
    (r"siunits(1 poise)", r"0.1 kg/(m*s)"),
    (r"siunits(1 stokes)", r"0.0001 m^2/s"),
    (r"siunits(1 planck)", r"6.62607015e-34 kg*m^2/s"),
    (
        r"siunits(1 hbar)",
        r"(132521403/(400000000000000000000000000000000000000000pi)) kg*m^2/s|  ~ 1.0545718176461563912624280033023e-34 kg*m^2/s",
    ),
    (r"siunits(1 k_B)", r"1.380649e-23 kg*m^2/(K*s^2)"),
    (r"siunits(1 N_A)", r"6.02214076e23 mol^-1"),
    (r"siunits(1 qe)", r"1.602176634e-19 A*s"),
    (r"siunits(1 Rgas)", r"8.31446261815324 kg*m^2/(K*mol*s^2)"),
    (r"siunits(1 faraday)", r"96485.3321233100184 A*s/mol"),
    (
        r"siunits(1 sigmaSB)",
        r"(5454781984210512994952000000pi^5/29438455734650141042413712126365436049) kg/(K^4*s^3)|  ~ 5.6703744191844294539709967318892e-8 kg/(K^4*s^3)",
    ),
    (r"siunits(1 G)", r"6.6743e-11 m^3/(kg*s^2)"),
    (r"siunits(1 me)", r"9.1093837139e-31 kg"),
    (r"siunits(1 mp)", r"1.67262192595e-27 kg"),
    (r"siunits(1 mn)", r"1.67492750056e-27 kg"),
    (r"siunits(1 mu0)", r"0.00000125663706127 kg*m/(A^2*s^2)"),
    (r"siunits(1 eps0)", r"8.8541878188e-12 A^2*s^4/(kg*m^3)"),
    (r"siunits(1 alphafs)", r"0.0072973525643"),
    (r"siunits(1 a0)", r"5.29177210544e-11 m"),
    (r"siunits(1 Msun)", r"1.98847e30 kg"),
    (r"siunits(1 Mearth)", r"5.9722e24 kg"),
    (r"siunits(1 Rsun)", r"695700000 m"),
    (r"siunits(1 Rearth)", r"6.371e6 m"),
];

const UNIT_CONVERSIONS: &[(&str, &str)] = &[
    (
        r"convert(100 km/h, m/s)",
        r"(250/9) m/s|  ~ 27.777777777777777777777777777778 m/s",
    ),
    (r"convert(60 mph, km/h)", r"96.56064 km/h"),
    (r"convert(1 mi, km)", r"1.609344 km"),
    (r"convert(5 ft + 3 inch, cm)", r"160.02 cm"),
    (r"convert(1 kWh, MJ)", r"3.6 MJ"),
    (
        r"convert(1 atm, psi)",
        r"(18677382000000/1270920461503) psi|  ~ 14.695948775513448725503472157155 psi",
    ),
    (r"convert(1 hp, W)", r"745.69987158227022 W"),
    (r"convert(1 gal, l)", r"3.785411784 l"),
    (r"convert(1 lb, g)", r"453.59237 g"),
    (r"convert(1 oz, g)", r"28.349523125 g"),
    (r"convert(1 nmi, m)", r"1852 m"),
    (r"convert(1 kn, km/h)", r"1.852 km/h"),
    (
        r"convert(1 ly, au)",
        r"(431996825232/6830953) au|  ~ 63241.077084266280268653583182317 au",
    ),
    (
        r"convert(1 pc, ly)",
        r"(10246429500/(999992651pi)) ly|  ~ 3.2615637771674335621386397070455 ly",
    ),
    (
        r"convert(1 J, eV)",
        r"(5000000000000000000000000000/801088317) eV|  ~ 6241509074460762607.7762409809304 eV",
    ),
    (r"convert(1 cal, J)", r"4.184 J"),
    (r"convert(1 BTU, kJ)", r"1.05505585262 kJ"),
    (
        r"convert(1 bar, atm)",
        r"(4000/4053) atm|  ~ 0.98692326671601283000246730816679 atm",
    ),
    (r"convert(760 torr, Pa)", r"101325 Pa"),
    (r"convert(1 yd, m)", r"0.9144 m"),
    (r"convert(1 acre, ha)", r"0.40468564224 ha"),
    (r"convert(1 d, s)", r"86400 s"),
    (r"convert(1 wk, h)", r"168 h"),
    (r"convert(1 yr, d)", r"365.25 d"),
    (r"convert(1 GiB, MB)", r"1073.741824 MB"),
    (r"convert(1 MiB, bit)", r"8388608 bit"),
    (
        r"convert(3000 rpm, rad/s)",
        r"(100pi) rad/s|  ~ 314.15926535897932384626433832795 rad/s",
    ),
    (
        r"convert(90 deg, rad)",
        r"(pi/2) rad|  ~ 1.5707963267948966192313216916398 rad",
    ),
    (r"convert(1 rev, deg)", r"360 deg"),
    (r"convert(1 arcmin, arcsec)", r"60 arcsec"),
    (r"convert(1 T, gauss)", r"10000 gauss"),
    (r"convert(1 Ah, C)", r"3600 C"),
    (r"convert(1 u, kg)", r"1.66053906892e-27 kg"),
    (
        r"convert(1 eV/k_B, K)",
        r"(16021766340/1380649) K|  ~ 11604.518121550082606078735435292 K",
    ),
    (
        r"convert(planck*c/(500 nm), eV)",
        r"(6621486190496429/2670294390000000) eV|  ~ 2.4796839686640052447550548911575 eV",
    ),
    (
        r"convert(1 mol*Rgas*300 K/(1 atm), l)",
        r"(207861565453831/8443750000000) l|  ~ 24.617209824287905255366395262768 l",
    ),
    (r"convert(c/(100 MHz), m)", r"2.99792458 m"),
    (r"convert(12 V*2 A, W)", r"24 W"),
    (r"convert(5 kg*gn, N)", r"49.03325 N"),
    (
        r"convert(1 slug, kg)",
        r"(8896443230521/609600000000) kg|  ~ 14.593902937206364829396325459318 kg",
    ),
    (r"convert(1 ton, kg)", r"907.18474 kg"),
    (r"convert(1 st, lb)", r"14 lb"),
    (r"convert(1 floz, ml)", r"29.5735295625 ml"),
    (r"convert(1 bbl, gal)", r"42 gal"),
    (r"convert(1 mmHg, Pa)", r"133.322387415 Pa"),
    (r"convert(1 kgf, N)", r"9.80665 N"),
    (r"convert(1 lbf, N)", r"4.4482216152605 N"),
    (r"convert(1 ftlbf, J)", r"1.3558179483314004 J"),
    (r"convert(1 M, mol/m^3)", r"1000 mol/m^3"),
    (r"convert(1 Ci, Bq)", r"37000000000 Bq"),
    (r"convert(G*Msun/(1 au)^2, m/s^2)", r"0.00593026 m/s^2"),
    (
        r"convert(sqrt(2 m^2), cm)",
        r"(100sqrt(2)) cm|  ~ 141.42135623730950488016887242097 cm",
    ),
    (r"convert(0 degC, degF)", r"32 degF"),
    (r"convert(100 degC, degF)", r"212 degF"),
    (r"convert(-40 degC, degF)", r"-40 degF"),
    (r"convert(98.6 degF, degC)", r"37 degC"),
    (r"convert(0 K, degC)", r"-273.15 degC"),
    (r"convert(25 degC, K)", r"298.15 K"),
    (
        r"convert(451 degF, K)",
        r"(91067/180) K|  ~ 505.92777777777777777777777777778 K",
    ),
    (r"convert(0 degF, degR)", r"459.67 degR"),
    (r"convert(300 K, degR)", r"540 degR"),
    (r"convert(10 degC - 5 degC, K)", r"5 K"),
    (r"convert(5 km, m)", r"5000 m"),
    (
        r"convert(1 m, in)",
        r"(5000/127) in|  ~ 39.37007874015748031496062992126 in",
    ),
    (r"convert(3 in, mm)", r"76.2 mm"),
    (r"convert(2 in^2, cm^2)", r"12.9032 cm^2"),
    (r"convert(12 in * 3 ft, m^2)", r"0.27870912 m^2"),
    (r"convert(100 min, [h, min])", r"[1 h, 40 min]"),
    (r"convert(3.7 days, [d, h, min])", r"[3 d, 16 h, 48 min]"),
    (r"convert(1 yr, [wk, d, h])", r"[52 wk, 1 d, 6 h]"),
    (r"convert(1.5 h, [h, min, s])", r"[1 h, 30 min, 0 s]"),
    (r"convert(10000 s, [h, min, s])", r"[2 h, 46 min, 40 s]"),
    (r"convert(5 ft + 3 in, [m, cm])", r"[1 m, 60.02 cm]"),
    (r"convert(0 km, m)", r"0 m"),
    (r"convert(0 degC, K)", r"273.15 K"),
    (r"convert((2 - 2) degC, degF)", r"32 degF"),
    (r"convert(-40 degF, degC)", r"-40 degC"),
    (r"convert(1 J/degC, J/K)", r"1 J/K"),
    (r"convert(1 J/(g*degC), J/(kg*K))", r"1000 J/(K*kg)"),
    (r"convert(pi/2 rad, deg)", r"90 deg"),
    (
        r"convert(1 rev, rad)",
        r"(2pi) rad|  ~ 6.283185307179586476925286766559 rad",
    ),
    (r"convert(60 mi/h, km/h)", r"96.56064 km/h"),
    (r"convert(2 km * 3 m, m^2)", r"6000 m^2"),
    (r"convert(1 l, cm^3)", r"1000 cm^3"),
    (r"convert(1 ha, m^2)", r"10000 m^2"),
    (r"convert(1 N, kg*m/s^2)", r"1 kg*m/s^2"),
    (r"convert(1 J, kg*m^2/s^2)", r"1 kg*m^2/s^2"),
    (r"convert(1 W, J/s)", r"1 J/s"),
    (r"convert(1 Pa, N/m^2)", r"1 N/m^2"),
    (r"convert(1 V, W/A)", r"1 W/A"),
    (r"convert(1 ohm, V/A)", r"1 V/A"),
    (r"convert(1 F, C/V)", r"1 C/V"),
    (r"convert(1 H, Wb/A)", r"1 Wb/A"),
    (r"convert(1 T, Wb/m^2)", r"1 Wb/m^2"),
    (r"convert(1 Hz, s^-1)", r"1 s^-1"),
    (r"convert(1 kWh, J)", r"3600000 J"),
    (r"convert(1 MeV, J)", r"1.602176634e-13 J"),
    (r"convert(1 kcal, kJ)", r"4.184 kJ"),
    (r"convert(1 mmol, mol)", r"0.001 mol"),
    (r"convert(1 um, nm)", r"1000 nm"),
    (r"convert(1 kilometer, meters)", r"1000 meters"),
    (r"convert(3 megawatts, kW)", r"3000 kW"),
    (r"convert(2 milliseconds, us)", r"2000 us"),
    (r"convert(1 kibibyte, bytes)", r"1024 bytes"),
    (r"convert(1 MiB, kB)", r"1048.576 kB"),
    (r"convert(8 bit, B)", r"1 B"),
    (r"convert(100 Mbps * 1 min, GB)", r"0.75 GB"),
    (r"convert(1 c, km/s)", r"299792.458 km/s"),
    (r"convert(1 lightyear, km)", r"9460730472580.8 km"),
    (r"convert(c*1 yr, ly)", r"1 ly"),
    (r"convert(1 eV, J)", r"1.602176634e-19 J"),
    (r"convert(planck*1 Hz, J)", r"6.62607015e-34 J"),
    (
        r"convert(hbar, J*s)",
        r"(132521403/(400000000000000000000000000000000000000000pi)) J*s|  ~ 1.0545718176461563912624280033023e-34 J*s",
    ),
    (
        r"convert(k_B*300 K, eV)",
        r"(6903245/267029439) eV|  ~ 0.025851999786435532300990978002242 eV",
    ),
    (r"convert(N_A*qe, C/mol)", r"96485.3321233100184 C/mol"),
    (r"convert(Rgas, J/(mol*K))", r"8.31446261815324 J/(K*mol)"),
    (
        r"convert(sigmaSB*(5772 K)^4, W/m^2)",
        r"(74747671495933630976056773530519552000000pi^5/363437725119137543733502618844017729) W/m^2|  ~ 62938592.470335950467548474587303 W/m^2",
    ),
    (r"convert(G*Mearth/Rearth^2, m/s^2)", r"9.82 m/s^2"),
    (r"convert(sqrt(2*G*Mearth/Rearth), km/s)", r"11.19 km/s"),
    (r"convert(me*c^2, MeV)", r"0.51099895069 MeV"),
    (r"convert(mp/me, percent)", r"183615.26734 percent"),
    (
        r"convert(1/(mu0*eps0), m^2/s^2)",
        r"8.9875517874e16 m^2/s^2",
    ),
    (r"convert(a0, pm)", r"52.9177210544 pm"),
    (r"convert(alphafs, ppm)", r"7297.3525643 ppm"),
    (r"convert(1 percent, ppm)", r"10000 ppm"),
    (
        r"convert(x km, m)",
        r"(1000x) m|note: x is taken as a number, not as a unit",
    ),
    (
        r"convert(x km + y m, cm)",
        r"(100000x + 100y) cm|note: x is taken as a number, not as a unit|note: y is taken as a number, not as a unit",
    ),
    (
        r"convert(a*5 m/s, km/h)",
        r"(18a) km/h|note: a is taken as a number, not as a unit",
    ),
    (r"convert((3 m)^2, cm^2)", r"90000 cm^2"),
    (r"convert(sin(30 deg) m, cm)", r"50 cm"),
    (r"convert(5 km, m)*2", r"10000 m"),
    (r"convert(5 km, m) + convert(3 mi, m)", r"9828.032 m"),
    (
        r"convert(5 km, m)/convert(2 h, s)",
        r"(25/36) m/s|  ~ 0.69444444444444444444444444444444 m/s",
    ),
    (r"convert(2 m, cm)^2", r"4 m^2"),
    (r"convert(1 h, min) - convert(600 s, min)", r"3000 s"),
    (
        r"convert(quantity(5, km), mi)",
        r"(78125/25146) mi|  ~ 3.1068559611866698480871709218166 mi",
    ),
    (r"quantity(5, km)", r"5 km"),
    (
        r"quantity(1/3, m/s)",
        r"(1/3) m/s|  ~ 0.33333333333333333333333333333333 m/s",
    ),
    (r"quantity(2.5, kg)", r"2.5 kg"),
    (
        r"quantity(x, s)",
        r"x s|note: x is taken as a number, not as a unit",
    ),
    (
        r"quantity(5 m, km)",
        r"error: quantity(value, unit): 5 m has a unit already",
    ),
    (r"siunits(60 mph)", r"26.8224 m/s"),
    (r"siunits(1 N*m)", r"1 kg*m^2/s^2"),
    (r"siunits(5 kg*gn)", r"49.03325 kg*m/s^2"),
    (r"siunits(1/(5 s))", r"0.2 s^-1"),
    (
        r"siunits(3 dimensionless)",
        r"3dimensionless|note: dimensionless is taken as a number, not as a unit",
    ),
    (r"siunits(12)", r"12"),
    (
        r"siunits(x km)",
        r"(1000x) m|note: x is taken as a number, not as a unit",
    ),
    (
        r"siunits(100 km/h)",
        r"(250/9) m/s|  ~ 27.777777777777777777777777777778 m/s",
    ),
    (r"siunits(1 kWh/(100 km))", r"36 kg*m/s^2"),
    (r"dimension(5 kWh)", r"length^2*mass/time^2"),
    (r"dimension(3 m/s^2)", r"length/time^2"),
    (r"dimension(planck)", r"length^2*mass/time"),
    (r"dimension(G)", r"length^3/(mass*time^2)"),
    (r"dimension(1 V/A)", r"length^2*mass/(current^2*time^3)"),
    (r"dimension(1 GB/s)", r"information/time"),
    (r"dimension(12)", r"1"),
    (r"dimension(1 rad)", r"1"),
    (r"dimension(sqrt(1 m^2/s))", r"length/sqrt(time)"),
    (r"m := 7; convert(3 m, cm); m", r"7|300 cm|7"),
    (r"h := 2; s := 3; convert(1 h, s); h*s", r"2|3|3600 s|6"),
    (r"convert(5 m, s)", r"error: 5 m is length, s is time"),
    (
        r"convert(5 kmm, m)",
        r"error: kmm is no unit: 5kmm is a plain number, m is length",
    ),
    (
        r"convert(5 m + 3 s, m)",
        r"error: time and length do not add up",
    ),
    (
        r"convert(5 km)",
        r"error: convert takes 2 argument(s), 1 given",
    ),
    (
        r"convert(1 degC + 1 m, K)",
        r"error: temperature and length do not add up",
    ),
    (
        r"convert(2^(3 m), m)",
        r"error: the exponent 3 m has a unit",
    ),
    (
        r"convert(sin(3 m), m)",
        r"error: sin of length: it takes a plain number",
    ),
    (
        r"convert([1 m, 2 m], cm)",
        r"error: [1 m, 2 m] is no quantity",
    ),
    (
        r"convert(1 lb/in^2, psi)",
        r"error: lb/in^2 is mass/length^2, psi is mass/(length*time^2)",
    ),
    (r"convert(1 lbf/in^2, psi)", r"1 psi"),
    (r"siunits()", r"error: siunits takes 1 argument(s), 0 given"),
    (
        r"dimension(1 m, 2)",
        r"error: dimension takes 1 argument(s), 2 given",
    ),
    (r"5 J", r"5i"),
    (r"3 km/h", r"3km/h"),
];

const GRAPHS: &[(&str, &str)] = &[
    (r"vertexlist([[1,2],[2,3],[3,1],[3,4]])", r"[1, 2, 3, 4]"),
    (r"vertexlist([[1,2],[3,4],[5]])", r"[1, 2, 3, 4, 5]"),
    (
        r"vertexlist([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[a, b, c, d]",
    ),
    (
        r"vertexlist([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1, 2, 3, 4]",
    ),
    (
        r"vertexlist([a -> b, b -> c, a -> c, d -> a])",
        r"[a, b, c, d]",
    ),
    (r"vertexlist([[1,2],[1,2],[2,2],[2,3]])", r"[1, 2, 3]"),
    (r"vertexlist(cyclegraph(5))", r"[1, 2, 3, 4, 5]"),
    (
        r"vertexlist(petersengraph())",
        r"[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]",
    ),
    (
        r"vertexlist(completebipartite(3, 3))",
        r"[1, 2, 3, 4, 5, 6]",
    ),
    (r"vertexlist([])", r"[]"),
    (r"vertexlist([[1]])", r"[1]"),
    (
        r"edgelist([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 2], [2, 3], [3, 1], [3, 4]]",
    ),
    (r"edgelist([[1,2],[3,4],[5]])", r"[[1, 2], [3, 4]]"),
    (
        r"edgelist([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[a, b, 7], [b, c, 1], [a, c, 3], [c, d, 2]]",
    ),
    (
        r"edgelist([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4]",
    ),
    (
        r"edgelist([a -> b, b -> c, a -> c, d -> a])",
        r"[a -> b, b -> c, a -> c, d -> a]",
    ),
    (
        r"edgelist([[1,2],[1,2],[2,2],[2,3]])",
        r"[[1, 2], [1, 2], [2, 2], [2, 3]]",
    ),
    (
        r"edgelist(cyclegraph(5))",
        r"[[1, 2], [2, 3], [3, 4], [4, 5], [5, 1]]",
    ),
    (
        r"edgelist(petersengraph())",
        r"[[1, 2], [2, 3], [3, 4], [4, 5], [5, 1], [1, 6], [2, 7], [3, 8], [4, 9], [5, 10], [6, 8], [7, 9], [8, 10], [9, 6], [10, 7]]",
    ),
    (
        r"edgelist(completebipartite(3, 3))",
        r"[[1, 4], [1, 5], [1, 6], [2, 4], [2, 5], [2, 6], [3, 4], [3, 5], [3, 6]]",
    ),
    (r"edgelist([])", r"[]"),
    (r"edgelist([[1]])", r"[]"),
    (r"vertexcount([[1,2],[2,3],[3,1],[3,4]])", r"4"),
    (r"vertexcount([[1,2],[3,4],[5]])", r"5"),
    (r"vertexcount([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"4"),
    (r"vertexcount([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"4"),
    (r"vertexcount([a -> b, b -> c, a -> c, d -> a])", r"4"),
    (r"vertexcount([[1,2],[1,2],[2,2],[2,3]])", r"3"),
    (r"vertexcount(cyclegraph(5))", r"5"),
    (r"vertexcount(petersengraph())", r"10"),
    (r"vertexcount(completebipartite(3, 3))", r"6"),
    (r"vertexcount([])", r"0"),
    (r"vertexcount([[1]])", r"1"),
    (r"edgecount([[1,2],[2,3],[3,1],[3,4]])", r"4"),
    (r"edgecount([[1,2],[3,4],[5]])", r"2"),
    (r"edgecount([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"4"),
    (r"edgecount([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"4"),
    (r"edgecount([a -> b, b -> c, a -> c, d -> a])", r"4"),
    (r"edgecount([[1,2],[1,2],[2,2],[2,3]])", r"4"),
    (r"edgecount(cyclegraph(5))", r"5"),
    (r"edgecount(petersengraph())", r"15"),
    (r"edgecount(completebipartite(3, 3))", r"9"),
    (r"edgecount([])", r"0"),
    (r"edgecount([[1]])", r"0"),
    (
        r"adjacencymatrix([[1,2],[2,3],[3,1],[3,4]])",
        r"[[0, 1, 1, 0], [1, 0, 1, 0], [1, 1, 0, 1], [0, 0, 1, 0]]",
    ),
    (
        r"adjacencymatrix([[1,2],[3,4],[5]])",
        r"[[0, 1, 0, 0, 0], [1, 0, 0, 0, 0], [0, 0, 0, 1, 0], [0, 0, 1, 0, 0], [0, 0, 0, 0, 0]]",
    ),
    (
        r"adjacencymatrix([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[0, 1, 1, 0], [1, 0, 1, 0], [1, 1, 0, 1], [0, 0, 1, 0]]",
    ),
    (
        r"adjacencymatrix([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[0, 1, 0, 0], [0, 0, 1, 0], [1, 0, 0, 1], [0, 0, 0, 0]]",
    ),
    (
        r"adjacencymatrix([a -> b, b -> c, a -> c, d -> a])",
        r"[[0, 1, 1, 0], [0, 0, 1, 0], [0, 0, 0, 0], [1, 0, 0, 0]]",
    ),
    (
        r"adjacencymatrix([[1,2],[1,2],[2,2],[2,3]])",
        r"[[0, 2, 0], [2, 1, 1], [0, 1, 0]]",
    ),
    (
        r"adjacencymatrix(cyclegraph(5))",
        r"[[0, 1, 0, 0, 1], [1, 0, 1, 0, 0], [0, 1, 0, 1, 0], [0, 0, 1, 0, 1], [1, 0, 0, 1, 0]]",
    ),
    (
        r"adjacencymatrix(petersengraph())",
        r"[[0, 1, 0, 0, 1, 1, 0, 0, 0, 0], [1, 0, 1, 0, 0, 0, 1, 0, 0, 0], [0, 1, 0, 1, 0, 0, 0, 1, 0, 0], [0, 0, 1, 0, 1, 0, 0, 0, 1, 0], [1, 0, 0, 1, 0, 0, 0, 0, 0, 1], [1, 0, 0, 0, 0, 0, 0, 1, 1, 0], [0, 1, 0, 0, 0, 0, 0, 0, 1, 1], [0, 0, 1, 0, 0, 1, 0, 0, 0, 1], [0, 0, 0, 1, 0, 1, 1, 0, 0, 0], [0, 0, 0, 0, 1, 0, 1, 1, 0, 0]]",
    ),
    (
        r"adjacencymatrix(completebipartite(3, 3))",
        r"[[0, 0, 0, 1, 1, 1], [0, 0, 0, 1, 1, 1], [0, 0, 0, 1, 1, 1], [1, 1, 1, 0, 0, 0], [1, 1, 1, 0, 0, 0], [1, 1, 1, 0, 0, 0]]",
    ),
    (r"adjacencymatrix([])", r"[]"),
    (r"adjacencymatrix([[1]])", r"[[0]]"),
    (
        r"weightmatrix([[1,2],[2,3],[3,1],[3,4]])",
        r"[[0, 1, 1, 0], [1, 0, 1, 0], [1, 1, 0, 1], [0, 0, 1, 0]]",
    ),
    (
        r"weightmatrix([[1,2],[3,4],[5]])",
        r"[[0, 1, 0, 0, 0], [1, 0, 0, 0, 0], [0, 0, 0, 1, 0], [0, 0, 1, 0, 0], [0, 0, 0, 0, 0]]",
    ),
    (
        r"weightmatrix([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[0, 7, 3, 0], [7, 0, 1, 0], [3, 1, 0, 2], [0, 0, 2, 0]]",
    ),
    (
        r"weightmatrix([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[0, 1, 0, 0], [0, 0, 1, 0], [1, 0, 0, 1], [0, 0, 0, 0]]",
    ),
    (
        r"weightmatrix([a -> b, b -> c, a -> c, d -> a])",
        r"[[0, 1, 1, 0], [0, 0, 1, 0], [0, 0, 0, 0], [1, 0, 0, 0]]",
    ),
    (
        r"weightmatrix([[1,2],[1,2],[2,2],[2,3]])",
        r"[[0, 2, 0], [2, 1, 1], [0, 1, 0]]",
    ),
    (
        r"weightmatrix(cyclegraph(5))",
        r"[[0, 1, 0, 0, 1], [1, 0, 1, 0, 0], [0, 1, 0, 1, 0], [0, 0, 1, 0, 1], [1, 0, 0, 1, 0]]",
    ),
    (
        r"weightmatrix(petersengraph())",
        r"[[0, 1, 0, 0, 1, 1, 0, 0, 0, 0], [1, 0, 1, 0, 0, 0, 1, 0, 0, 0], [0, 1, 0, 1, 0, 0, 0, 1, 0, 0], [0, 0, 1, 0, 1, 0, 0, 0, 1, 0], [1, 0, 0, 1, 0, 0, 0, 0, 0, 1], [1, 0, 0, 0, 0, 0, 0, 1, 1, 0], [0, 1, 0, 0, 0, 0, 0, 0, 1, 1], [0, 0, 1, 0, 0, 1, 0, 0, 0, 1], [0, 0, 0, 1, 0, 1, 1, 0, 0, 0], [0, 0, 0, 0, 1, 0, 1, 1, 0, 0]]",
    ),
    (
        r"weightmatrix(completebipartite(3, 3))",
        r"[[0, 0, 0, 1, 1, 1], [0, 0, 0, 1, 1, 1], [0, 0, 0, 1, 1, 1], [1, 1, 1, 0, 0, 0], [1, 1, 1, 0, 0, 0], [1, 1, 1, 0, 0, 0]]",
    ),
    (r"weightmatrix([])", r"[]"),
    (r"weightmatrix([[1]])", r"[[0]]"),
    (
        r"incidencematrix([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 0, 1, 0], [1, 1, 0, 0], [0, 1, 1, 1], [0, 0, 0, 1]]",
    ),
    (
        r"incidencematrix([[1,2],[3,4],[5]])",
        r"[[1, 0], [1, 0], [0, 1], [0, 1], [0, 0]]",
    ),
    (
        r"incidencematrix([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[1, 0, 1, 0], [1, 1, 0, 0], [0, 1, 1, 1], [0, 0, 0, 1]]",
    ),
    (
        r"incidencematrix([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[-1, 0, 1, 0], [1, -1, 0, 0], [0, 1, -1, -1], [0, 0, 0, 1]]",
    ),
    (
        r"incidencematrix([a -> b, b -> c, a -> c, d -> a])",
        r"[[-1, 0, -1, 1], [1, -1, 0, 0], [0, 1, 1, 0], [0, 0, 0, -1]]",
    ),
    (
        r"incidencematrix([[1,2],[1,2],[2,2],[2,3]])",
        r"[[1, 1, 0, 0], [1, 1, 2, 1], [0, 0, 0, 1]]",
    ),
    (
        r"incidencematrix(cyclegraph(5))",
        r"[[1, 0, 0, 0, 1], [1, 1, 0, 0, 0], [0, 1, 1, 0, 0], [0, 0, 1, 1, 0], [0, 0, 0, 1, 1]]",
    ),
    (
        r"incidencematrix(petersengraph())",
        r"[[1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0], [1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0], [0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0], [0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0], [0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0], [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1], [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0], [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0], [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1]]",
    ),
    (
        r"incidencematrix(completebipartite(3, 3))",
        r"[[1, 1, 1, 0, 0, 0, 0, 0, 0], [0, 0, 0, 1, 1, 1, 0, 0, 0], [0, 0, 0, 0, 0, 0, 1, 1, 1], [1, 0, 0, 1, 0, 0, 1, 0, 0], [0, 1, 0, 0, 1, 0, 0, 1, 0], [0, 0, 1, 0, 0, 1, 0, 0, 1]]",
    ),
    (r"incidencematrix([])", r"[]"),
    (r"incidencematrix([[1]])", r"[[]]"),
    (
        r"laplacianmatrix([[1,2],[2,3],[3,1],[3,4]])",
        r"[[2, -1, -1, 0], [-1, 2, -1, 0], [-1, -1, 3, -1], [0, 0, -1, 1]]",
    ),
    (
        r"laplacianmatrix([[1,2],[3,4],[5]])",
        r"[[1, -1, 0, 0, 0], [-1, 1, 0, 0, 0], [0, 0, 1, -1, 0], [0, 0, -1, 1, 0], [0, 0, 0, 0, 0]]",
    ),
    (
        r"laplacianmatrix([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[2, -1, -1, 0], [-1, 2, -1, 0], [-1, -1, 3, -1], [0, 0, -1, 1]]",
    ),
    (
        r"laplacianmatrix([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[1, -1, 0, 0], [0, 1, -1, 0], [-1, 0, 2, -1], [0, 0, 0, 0]]",
    ),
    (
        r"laplacianmatrix([a -> b, b -> c, a -> c, d -> a])",
        r"[[2, -1, -1, 0], [0, 1, -1, 0], [0, 0, 0, 0], [-1, 0, 0, 1]]",
    ),
    (
        r"laplacianmatrix([[1,2],[1,2],[2,2],[2,3]])",
        r"[[2, -2, 0], [-2, 3, -1], [0, -1, 1]]",
    ),
    (
        r"laplacianmatrix(cyclegraph(5))",
        r"[[2, -1, 0, 0, -1], [-1, 2, -1, 0, 0], [0, -1, 2, -1, 0], [0, 0, -1, 2, -1], [-1, 0, 0, -1, 2]]",
    ),
    (
        r"laplacianmatrix(petersengraph())",
        r"[[3, -1, 0, 0, -1, -1, 0, 0, 0, 0], [-1, 3, -1, 0, 0, 0, -1, 0, 0, 0], [0, -1, 3, -1, 0, 0, 0, -1, 0, 0], [0, 0, -1, 3, -1, 0, 0, 0, -1, 0], [-1, 0, 0, -1, 3, 0, 0, 0, 0, -1], [-1, 0, 0, 0, 0, 3, 0, -1, -1, 0], [0, -1, 0, 0, 0, 0, 3, 0, -1, -1], [0, 0, -1, 0, 0, -1, 0, 3, 0, -1], [0, 0, 0, -1, 0, -1, -1, 0, 3, 0], [0, 0, 0, 0, -1, 0, -1, -1, 0, 3]]",
    ),
    (
        r"laplacianmatrix(completebipartite(3, 3))",
        r"[[3, 0, 0, -1, -1, -1], [0, 3, 0, -1, -1, -1], [0, 0, 3, -1, -1, -1], [-1, -1, -1, 3, 0, 0], [-1, -1, -1, 0, 3, 0], [-1, -1, -1, 0, 0, 3]]",
    ),
    (r"laplacianmatrix([])", r"[]"),
    (r"laplacianmatrix([[1]])", r"[[0]]"),
    (r"degrees([[1,2],[2,3],[3,1],[3,4]])", r"[2, 2, 3, 1]"),
    (r"degrees([[1,2],[3,4],[5]])", r"[1, 1, 1, 1, 0]"),
    (
        r"degrees([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[2, 2, 3, 1]",
    ),
    (
        r"degrees([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[2, 2, 3, 1]",
    ),
    (
        r"degrees([a -> b, b -> c, a -> c, d -> a])",
        r"[3, 2, 2, 1]",
    ),
    (r"degrees([[1,2],[1,2],[2,2],[2,3]])", r"[2, 5, 1]"),
    (r"degrees(cyclegraph(5))", r"[2, 2, 2, 2, 2]"),
    (
        r"degrees(petersengraph())",
        r"[3, 3, 3, 3, 3, 3, 3, 3, 3, 3]",
    ),
    (r"degrees(completebipartite(3, 3))", r"[3, 3, 3, 3, 3, 3]"),
    (r"degrees([])", r"[]"),
    (r"degrees([[1]])", r"[0]"),
    (r"indegrees([[1,2],[2,3],[3,1],[3,4]])", r"[2, 2, 3, 1]"),
    (r"indegrees([[1,2],[3,4],[5]])", r"[1, 1, 1, 1, 0]"),
    (
        r"indegrees([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[2, 2, 3, 1]",
    ),
    (
        r"indegrees([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1, 1, 1, 1]",
    ),
    (
        r"indegrees([a -> b, b -> c, a -> c, d -> a])",
        r"[1, 1, 2, 0]",
    ),
    (r"indegrees([[1,2],[1,2],[2,2],[2,3]])", r"[2, 4, 1]"),
    (r"indegrees(cyclegraph(5))", r"[2, 2, 2, 2, 2]"),
    (
        r"indegrees(petersengraph())",
        r"[3, 3, 3, 3, 3, 3, 3, 3, 3, 3]",
    ),
    (r"indegrees(completebipartite(3, 3))", r"[3, 3, 3, 3, 3, 3]"),
    (r"indegrees([])", r"[]"),
    (r"indegrees([[1]])", r"[0]"),
    (r"outdegrees([[1,2],[2,3],[3,1],[3,4]])", r"[2, 2, 3, 1]"),
    (r"outdegrees([[1,2],[3,4],[5]])", r"[1, 1, 1, 1, 0]"),
    (
        r"outdegrees([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[2, 2, 3, 1]",
    ),
    (
        r"outdegrees([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1, 1, 2, 0]",
    ),
    (
        r"outdegrees([a -> b, b -> c, a -> c, d -> a])",
        r"[2, 1, 0, 1]",
    ),
    (r"outdegrees([[1,2],[1,2],[2,2],[2,3]])", r"[2, 4, 1]"),
    (r"outdegrees(cyclegraph(5))", r"[2, 2, 2, 2, 2]"),
    (
        r"outdegrees(petersengraph())",
        r"[3, 3, 3, 3, 3, 3, 3, 3, 3, 3]",
    ),
    (
        r"outdegrees(completebipartite(3, 3))",
        r"[3, 3, 3, 3, 3, 3]",
    ),
    (r"outdegrees([])", r"[]"),
    (r"outdegrees([[1]])", r"[0]"),
    (r"components([[1,2],[2,3],[3,1],[3,4]])", r"[[1, 2, 3, 4]]"),
    (r"components([[1,2],[3,4],[5]])", r"[[1, 2], [3, 4], [5]]"),
    (
        r"components([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[a, b, c, d]]",
    ),
    (
        r"components([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[1, 2, 3, 4]]",
    ),
    (
        r"components([a -> b, b -> c, a -> c, d -> a])",
        r"[[a, b, c, d]]",
    ),
    (r"components([[1,2],[1,2],[2,2],[2,3]])", r"[[1, 2, 3]]"),
    (r"components(cyclegraph(5))", r"[[1, 2, 3, 4, 5]]"),
    (
        r"components(petersengraph())",
        r"[[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]]",
    ),
    (
        r"components(completebipartite(3, 3))",
        r"[[1, 2, 3, 4, 5, 6]]",
    ),
    (r"components([])", r"[]"),
    (r"components([[1]])", r"[[1]]"),
    (
        r"strongcomponents([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 2, 3, 4]]",
    ),
    (
        r"strongcomponents([[1,2],[3,4],[5]])",
        r"[[1, 2], [3, 4], [5]]",
    ),
    (
        r"strongcomponents([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[a, b, c, d]]",
    ),
    (
        r"strongcomponents([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[1, 2, 3], [4]]",
    ),
    (
        r"strongcomponents([a -> b, b -> c, a -> c, d -> a])",
        r"[[a], [b], [c], [d]]",
    ),
    (
        r"strongcomponents([[1,2],[1,2],[2,2],[2,3]])",
        r"[[1, 2, 3]]",
    ),
    (r"strongcomponents(cyclegraph(5))", r"[[1, 2, 3, 4, 5]]"),
    (
        r"strongcomponents(petersengraph())",
        r"[[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]]",
    ),
    (
        r"strongcomponents(completebipartite(3, 3))",
        r"[[1, 2, 3, 4, 5, 6]]",
    ),
    (r"strongcomponents([])", r"[]"),
    (r"strongcomponents([[1]])", r"[[1]]"),
    (r"isconnected([[1,2],[2,3],[3,1],[3,4]])", r"true"),
    (r"isconnected([[1,2],[3,4],[5]])", r"false"),
    (r"isconnected([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"true"),
    (r"isconnected([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"false"),
    (r"isconnected([a -> b, b -> c, a -> c, d -> a])", r"false"),
    (r"isconnected([[1,2],[1,2],[2,2],[2,3]])", r"true"),
    (r"isconnected(cyclegraph(5))", r"true"),
    (r"isconnected(petersengraph())", r"true"),
    (r"isconnected(completebipartite(3, 3))", r"true"),
    (r"isconnected([])", r"true"),
    (r"isconnected([[1]])", r"true"),
    (
        r"distancematrix([[1,2],[2,3],[3,1],[3,4]])",
        r"[[0, 1, 1, 2], [1, 0, 1, 2], [1, 1, 0, 1], [2, 2, 1, 0]]",
    ),
    (
        r"distancematrix([[1,2],[3,4],[5]])",
        r"[[0, 1, inf, inf, inf], [1, 0, inf, inf, inf], [inf, inf, 0, 1, inf], [inf, inf, 1, 0, inf], [inf, inf, inf, inf, 0]]",
    ),
    (
        r"distancematrix([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[0, 4, 3, 5], [4, 0, 1, 3], [3, 1, 0, 2], [5, 3, 2, 0]]",
    ),
    (
        r"distancematrix([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[0, 1, 2, 3], [2, 0, 1, 2], [1, 2, 0, 1], [inf, inf, inf, 0]]",
    ),
    (
        r"distancematrix([a -> b, b -> c, a -> c, d -> a])",
        r"[[0, 1, 1, inf], [inf, 0, 1, inf], [inf, inf, 0, inf], [1, 2, 2, 0]]",
    ),
    (
        r"distancematrix([[1,2],[1,2],[2,2],[2,3]])",
        r"[[0, 1, 2], [1, 0, 1], [2, 1, 0]]",
    ),
    (
        r"distancematrix(cyclegraph(5))",
        r"[[0, 1, 2, 2, 1], [1, 0, 1, 2, 2], [2, 1, 0, 1, 2], [2, 2, 1, 0, 1], [1, 2, 2, 1, 0]]",
    ),
    (
        r"distancematrix(petersengraph())",
        r"[[0, 1, 2, 2, 1, 1, 2, 2, 2, 2], [1, 0, 1, 2, 2, 2, 1, 2, 2, 2], [2, 1, 0, 1, 2, 2, 2, 1, 2, 2], [2, 2, 1, 0, 1, 2, 2, 2, 1, 2], [1, 2, 2, 1, 0, 2, 2, 2, 2, 1], [1, 2, 2, 2, 2, 0, 2, 1, 1, 2], [2, 1, 2, 2, 2, 2, 0, 2, 1, 1], [2, 2, 1, 2, 2, 1, 2, 0, 2, 1], [2, 2, 2, 1, 2, 1, 1, 2, 0, 2], [2, 2, 2, 2, 1, 2, 1, 1, 2, 0]]",
    ),
    (
        r"distancematrix(completebipartite(3, 3))",
        r"[[0, 2, 2, 1, 1, 1], [2, 0, 2, 1, 1, 1], [2, 2, 0, 1, 1, 1], [1, 1, 1, 0, 2, 2], [1, 1, 1, 2, 0, 2], [1, 1, 1, 2, 2, 0]]",
    ),
    (r"distancematrix([])", r"[]"),
    (r"distancematrix([[1]])", r"[[0]]"),
    (r"diameter([[1,2],[2,3],[3,1],[3,4]])", r"2"),
    (r"diameter([[1,2],[3,4],[5]])", r"inf"),
    (r"diameter([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"5"),
    (r"diameter([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"inf"),
    (r"diameter([a -> b, b -> c, a -> c, d -> a])", r"inf"),
    (r"diameter([[1,2],[1,2],[2,2],[2,3]])", r"2"),
    (r"diameter(cyclegraph(5))", r"2"),
    (r"diameter(petersengraph())", r"2"),
    (r"diameter(completebipartite(3, 3))", r"2"),
    (r"diameter([])", r"error: diameter: the graph has no vertex"),
    (r"diameter([[1]])", r"0"),
    (r"graphradius([[1,2],[2,3],[3,1],[3,4]])", r"1"),
    (r"graphradius([[1,2],[3,4],[5]])", r"inf"),
    (r"graphradius([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"3"),
    (r"graphradius([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"2"),
    (r"graphradius([a -> b, b -> c, a -> c, d -> a])", r"2"),
    (r"graphradius([[1,2],[1,2],[2,2],[2,3]])", r"1"),
    (r"graphradius(cyclegraph(5))", r"2"),
    (r"graphradius(petersengraph())", r"2"),
    (r"graphradius(completebipartite(3, 3))", r"2"),
    (
        r"graphradius([])",
        r"error: graphradius: the graph has no vertex",
    ),
    (r"graphradius([[1]])", r"0"),
    (r"graphcenter([[1,2],[2,3],[3,1],[3,4]])", r"[3]"),
    (r"graphcenter([[1,2],[3,4],[5]])", r"[1, 2, 3, 4, 5]"),
    (r"graphcenter([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"[c]"),
    (r"graphcenter([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"[2, 3]"),
    (r"graphcenter([a -> b, b -> c, a -> c, d -> a])", r"[d]"),
    (r"graphcenter([[1,2],[1,2],[2,2],[2,3]])", r"[2]"),
    (r"graphcenter(cyclegraph(5))", r"[1, 2, 3, 4, 5]"),
    (
        r"graphcenter(petersengraph())",
        r"[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]",
    ),
    (
        r"graphcenter(completebipartite(3, 3))",
        r"[1, 2, 3, 4, 5, 6]",
    ),
    (
        r"graphcenter([])",
        r"error: graphcenter: the graph has no vertex",
    ),
    (r"graphcenter([[1]])", r"[1]"),
    (r"girth([[1,2],[2,3],[3,1],[3,4]])", r"3"),
    (r"girth([[1,2],[3,4],[5]])", r"inf"),
    (r"girth([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"3"),
    (
        r"girth([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: girth is for graphs without directions",
    ),
    (
        r"girth([a -> b, b -> c, a -> c, d -> a])",
        r"error: girth is for graphs without directions",
    ),
    (r"girth([[1,2],[1,2],[2,2],[2,3]])", r"1"),
    (r"girth(cyclegraph(5))", r"5"),
    (r"girth(petersengraph())", r"5"),
    (r"girth(completebipartite(3, 3))", r"4"),
    (r"girth([])", r"inf"),
    (r"girth([[1]])", r"inf"),
    (
        r"spanningtree([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 2], [2, 3], [3, 4]]",
    ),
    (r"spanningtree([[1,2],[3,4],[5]])", r"[[1, 2], [3, 4], [5]]"),
    (
        r"spanningtree([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[b, c, 1], [a, c, 3], [c, d, 2]]",
    ),
    (
        r"spanningtree([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: spanningtree is for graphs without directions",
    ),
    (
        r"spanningtree([a -> b, b -> c, a -> c, d -> a])",
        r"error: spanningtree is for graphs without directions",
    ),
    (
        r"spanningtree([[1,2],[1,2],[2,2],[2,3]])",
        r"[[1, 2], [2, 3]]",
    ),
    (
        r"spanningtree(cyclegraph(5))",
        r"[[1, 2], [2, 3], [3, 4], [4, 5]]",
    ),
    (
        r"spanningtree(petersengraph())",
        r"[[1, 2], [2, 3], [3, 4], [4, 5], [1, 6], [2, 7], [3, 8], [4, 9], [5, 10]]",
    ),
    (
        r"spanningtree(completebipartite(3, 3))",
        r"[[1, 4], [1, 5], [1, 6], [2, 4], [3, 4]]",
    ),
    (r"spanningtree([])", r"[]"),
    (r"spanningtree([[1]])", r"[[1]]"),
    (r"spanningtreecount([[1,2],[2,3],[3,1],[3,4]])", r"3"),
    (r"spanningtreecount([[1,2],[3,4],[5]])", r"0"),
    (
        r"spanningtreecount([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"3",
    ),
    (
        r"spanningtreecount([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: spanningtreecount is for graphs without directions",
    ),
    (
        r"spanningtreecount([a -> b, b -> c, a -> c, d -> a])",
        r"error: spanningtreecount is for graphs without directions",
    ),
    (r"spanningtreecount([[1,2],[1,2],[2,2],[2,3]])", r"2"),
    (r"spanningtreecount(cyclegraph(5))", r"5"),
    (r"spanningtreecount(petersengraph())", r"2000"),
    (r"spanningtreecount(completebipartite(3, 3))", r"81"),
    (r"spanningtreecount([])", r"0"),
    (r"spanningtreecount([[1]])", r"1"),
    (r"isbipartite([[1,2],[2,3],[3,1],[3,4]])", r"false"),
    (r"isbipartite([[1,2],[3,4],[5]])", r"true"),
    (r"isbipartite([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"false"),
    (r"isbipartite([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"false"),
    (r"isbipartite([a -> b, b -> c, a -> c, d -> a])", r"false"),
    (r"isbipartite([[1,2],[1,2],[2,2],[2,3]])", r"false"),
    (r"isbipartite(cyclegraph(5))", r"false"),
    (r"isbipartite(petersengraph())", r"false"),
    (r"isbipartite(completebipartite(3, 3))", r"true"),
    (r"isbipartite([])", r"true"),
    (r"isbipartite([[1]])", r"true"),
    (
        r"bipartition([[1,2],[2,3],[3,1],[3,4]])",
        r"error: bipartition: the graph has an odd way around",
    ),
    (r"bipartition([[1,2],[3,4],[5]])", r"[[1, 3, 5], [2, 4]]"),
    (
        r"bipartition([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"error: bipartition: the graph has an odd way around",
    ),
    (
        r"bipartition([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: bipartition: the graph has an odd way around",
    ),
    (
        r"bipartition([a -> b, b -> c, a -> c, d -> a])",
        r"error: bipartition: the graph has an odd way around",
    ),
    (
        r"bipartition([[1,2],[1,2],[2,2],[2,3]])",
        r"error: bipartition: the graph has an odd way around",
    ),
    (
        r"bipartition(cyclegraph(5))",
        r"error: bipartition: the graph has an odd way around",
    ),
    (
        r"bipartition(petersengraph())",
        r"error: bipartition: the graph has an odd way around",
    ),
    (
        r"bipartition(completebipartite(3, 3))",
        r"[[1, 2, 3], [4, 5, 6]]",
    ),
    (r"bipartition([])", r"[[], []]"),
    (r"bipartition([[1]])", r"[[1], []]"),
    (r"istree([[1,2],[2,3],[3,1],[3,4]])", r"false"),
    (r"istree([[1,2],[3,4],[5]])", r"false"),
    (r"istree([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"false"),
    (r"istree([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"false"),
    (r"istree([a -> b, b -> c, a -> c, d -> a])", r"false"),
    (r"istree([[1,2],[1,2],[2,2],[2,3]])", r"false"),
    (r"istree(cyclegraph(5))", r"false"),
    (r"istree(petersengraph())", r"false"),
    (r"istree(completebipartite(3, 3))", r"false"),
    (r"istree([])", r"false"),
    (r"istree([[1]])", r"true"),
    (r"isacyclic([[1,2],[2,3],[3,1],[3,4]])", r"false"),
    (r"isacyclic([[1,2],[3,4],[5]])", r"true"),
    (r"isacyclic([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"false"),
    (r"isacyclic([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"false"),
    (r"isacyclic([a -> b, b -> c, a -> c, d -> a])", r"true"),
    (r"isacyclic([[1,2],[1,2],[2,2],[2,3]])", r"false"),
    (r"isacyclic(cyclegraph(5))", r"false"),
    (r"isacyclic(petersengraph())", r"false"),
    (r"isacyclic(completebipartite(3, 3))", r"false"),
    (r"isacyclic([])", r"true"),
    (r"isacyclic([[1]])", r"true"),
    (r"findcycle([[1,2],[2,3],[3,1],[3,4]])", r"[1, 2, 3, 1]"),
    (r"findcycle([[1,2],[3,4],[5]])", r"[]"),
    (
        r"findcycle([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[a, b, c, a]",
    ),
    (
        r"findcycle([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1, 2, 3, 1]",
    ),
    (r"findcycle([a -> b, b -> c, a -> c, d -> a])", r"[]"),
    (r"findcycle([[1,2],[1,2],[2,2],[2,3]])", r"[2, 2]"),
    (r"findcycle(cyclegraph(5))", r"[1, 2, 3, 4, 5, 1]"),
    (r"findcycle(petersengraph())", r"[1, 2, 3, 4, 5, 1]"),
    (r"findcycle(completebipartite(3, 3))", r"[1, 4, 2, 5, 1]"),
    (r"findcycle([])", r"[]"),
    (r"findcycle([[1]])", r"[]"),
    (
        r"topologicalsort([[1,2],[2,3],[3,1],[3,4]])",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort([[1,2],[3,4],[5]])",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: topologicalsort: the graph has a way around",
    ),
    (
        r"topologicalsort([a -> b, b -> c, a -> c, d -> a])",
        r"[d, a, b, c]",
    ),
    (
        r"topologicalsort([[1,2],[1,2],[2,2],[2,3]])",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort(cyclegraph(5))",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort(petersengraph())",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort(completebipartite(3, 3))",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (r"topologicalsort([])", r"[]"),
    (r"topologicalsort([[1]])", r"[1]"),
    (
        r"chromaticpolynomial([[1,2],[2,3],[3,1],[3,4]])",
        r"x*(x - 1)^2*(x - 2)",
    ),
    (r"chromaticpolynomial([[1,2],[3,4],[5]])", r"x^3*(x - 1)^2"),
    (
        r"chromaticpolynomial([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"x*(x - 1)^2*(x - 2)",
    ),
    (
        r"chromaticpolynomial([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"x*(x - 1)^2*(x - 2)",
    ),
    (
        r"chromaticpolynomial([a -> b, b -> c, a -> c, d -> a])",
        r"x*(x - 1)^2*(x - 2)",
    ),
    (r"chromaticpolynomial([[1,2],[1,2],[2,2],[2,3]])", r"0"),
    (
        r"chromaticpolynomial(cyclegraph(5))",
        r"x^5 - 5x^4 + 10x^3 - 10x^2 + 4x",
    ),
    (
        r"chromaticpolynomial(petersengraph())",
        r"x*(x - 1)*(x - 2)*(x^7 - 12x^6 + 67x^5 - 230x^4 + 529x^3 - 814x^2 + 775x - 352)",
    ),
    (
        r"chromaticpolynomial(completebipartite(3, 3))",
        r"x^6 - 9x^5 + 36x^4 - 75x^3 + 78x^2 - 31x",
    ),
    (r"chromaticpolynomial([])", r"1"),
    (r"chromaticpolynomial([[1]])", r"x"),
    (r"chromaticnumber([[1,2],[2,3],[3,1],[3,4]])", r"3"),
    (r"chromaticnumber([[1,2],[3,4],[5]])", r"2"),
    (r"chromaticnumber([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"3"),
    (r"chromaticnumber([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"3"),
    (r"chromaticnumber([a -> b, b -> c, a -> c, d -> a])", r"3"),
    (
        r"chromaticnumber([[1,2],[1,2],[2,2],[2,3]])",
        r"error: a graph with a loop has no coloring",
    ),
    (r"chromaticnumber(cyclegraph(5))", r"3"),
    (r"chromaticnumber(petersengraph())", r"3"),
    (r"chromaticnumber(completebipartite(3, 3))", r"2"),
    (r"chromaticnumber([])", r"0"),
    (r"chromaticnumber([[1]])", r"1"),
    (
        r"graphcoloring([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 4], [2], [3]]",
    ),
    (r"graphcoloring([[1,2],[3,4],[5]])", r"[[1, 3, 5], [2, 4]]"),
    (
        r"graphcoloring([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[a, d], [b], [c]]",
    ),
    (
        r"graphcoloring([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[[1, 4], [2], [3]]",
    ),
    (
        r"graphcoloring([a -> b, b -> c, a -> c, d -> a])",
        r"[[a], [b, d], [c]]",
    ),
    (
        r"graphcoloring([[1,2],[1,2],[2,2],[2,3]])",
        r"error: a graph with a loop has no coloring",
    ),
    (r"graphcoloring(cyclegraph(5))", r"[[1, 3], [2, 4], [5]]"),
    (
        r"graphcoloring(petersengraph())",
        r"[[1, 3, 7], [2, 4, 6, 10], [5, 8, 9]]",
    ),
    (
        r"graphcoloring(completebipartite(3, 3))",
        r"[[1, 2, 3], [4, 5, 6]]",
    ),
    (r"graphcoloring([])", r"[]"),
    (r"graphcoloring([[1]])", r"[[1]]"),
    (r"iseulerian([[1,2],[2,3],[3,1],[3,4]])", r"false"),
    (r"iseulerian([[1,2],[3,4],[5]])", r"false"),
    (r"iseulerian([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"false"),
    (r"iseulerian([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"false"),
    (r"iseulerian([a -> b, b -> c, a -> c, d -> a])", r"false"),
    (r"iseulerian([[1,2],[1,2],[2,2],[2,3]])", r"false"),
    (r"iseulerian(cyclegraph(5))", r"true"),
    (r"iseulerian(petersengraph())", r"false"),
    (r"iseulerian(completebipartite(3, 3))", r"false"),
    (r"iseulerian([])", r"false"),
    (r"iseulerian([[1]])", r"false"),
    (
        r"eulerianpath([[1,2],[2,3],[3,1],[3,4]])",
        r"[3, 2, 1, 3, 4]",
    ),
    (r"eulerianpath([[1,2],[3,4],[5]])", r"[]"),
    (
        r"eulerianpath([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[c, b, a, c, d]",
    ),
    (
        r"eulerianpath([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[3, 1, 2, 3, 4]",
    ),
    (r"eulerianpath([a -> b, b -> c, a -> c, d -> a])", r"[]"),
    (
        r"eulerianpath([[1,2],[1,2],[2,2],[2,3]])",
        r"[2, 1, 2, 2, 3]",
    ),
    (r"eulerianpath(cyclegraph(5))", r"[1, 2, 3, 4, 5, 1]"),
    (r"eulerianpath(petersengraph())", r"[]"),
    (r"eulerianpath(completebipartite(3, 3))", r"[]"),
    (r"eulerianpath([])", r"[]"),
    (r"eulerianpath([[1]])", r"[]"),
    (r"ishamiltonian([[1,2],[2,3],[3,1],[3,4]])", r"false"),
    (r"ishamiltonian([[1,2],[3,4],[5]])", r"false"),
    (
        r"ishamiltonian([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"false",
    ),
    (r"ishamiltonian([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"false"),
    (r"ishamiltonian([a -> b, b -> c, a -> c, d -> a])", r"false"),
    (r"ishamiltonian([[1,2],[1,2],[2,2],[2,3]])", r"false"),
    (r"ishamiltonian(cyclegraph(5))", r"true"),
    (r"ishamiltonian(petersengraph())", r"false"),
    (r"ishamiltonian(completebipartite(3, 3))", r"true"),
    (r"ishamiltonian([])", r"false"),
    (r"ishamiltonian([[1]])", r"false"),
    (r"hamiltoniancycle([[1,2],[2,3],[3,1],[3,4]])", r"[]"),
    (r"hamiltoniancycle([[1,2],[3,4],[5]])", r"[]"),
    (
        r"hamiltoniancycle([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[]",
    ),
    (r"hamiltoniancycle([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"[]"),
    (r"hamiltoniancycle([a -> b, b -> c, a -> c, d -> a])", r"[]"),
    (r"hamiltoniancycle([[1,2],[1,2],[2,2],[2,3]])", r"[]"),
    (r"hamiltoniancycle(cyclegraph(5))", r"[1, 2, 3, 4, 5, 1]"),
    (r"hamiltoniancycle(petersengraph())", r"[]"),
    (
        r"hamiltoniancycle(completebipartite(3, 3))",
        r"[1, 4, 2, 5, 3, 6, 1]",
    ),
    (r"hamiltoniancycle([])", r"[]"),
    (r"hamiltoniancycle([[1]])", r"[]"),
    (
        r"hamiltonianpath([[1,2],[2,3],[3,1],[3,4]])",
        r"[1, 2, 3, 4]",
    ),
    (r"hamiltonianpath([[1,2],[3,4],[5]])", r"[]"),
    (
        r"hamiltonianpath([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[a, b, c, d]",
    ),
    (
        r"hamiltonianpath([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1, 2, 3, 4]",
    ),
    (
        r"hamiltonianpath([a -> b, b -> c, a -> c, d -> a])",
        r"[d, a, b, c]",
    ),
    (r"hamiltonianpath([[1,2],[1,2],[2,2],[2,3]])", r"[1, 2, 3]"),
    (r"hamiltonianpath(cyclegraph(5))", r"[1, 2, 3, 4, 5]"),
    (
        r"hamiltonianpath(petersengraph())",
        r"[1, 2, 3, 4, 5, 10, 7, 9, 6, 8]",
    ),
    (
        r"hamiltonianpath(completebipartite(3, 3))",
        r"[1, 4, 2, 5, 3, 6]",
    ),
    (r"hamiltonianpath([])", r"[]"),
    (r"hamiltonianpath([[1]])", r"[1]"),
    (
        r"maxmatching([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 2], [3, 4]]",
    ),
    (r"maxmatching([[1,2],[3,4],[5]])", r"[[1, 2], [3, 4]]"),
    (
        r"maxmatching([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[a, b], [c, d]]",
    ),
    (
        r"maxmatching([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: maxmatching is for graphs without directions",
    ),
    (
        r"maxmatching([a -> b, b -> c, a -> c, d -> a])",
        r"error: maxmatching is for graphs without directions",
    ),
    (r"maxmatching([[1,2],[1,2],[2,2],[2,3]])", r"[[1, 2]]"),
    (r"maxmatching(cyclegraph(5))", r"[[1, 2], [3, 4]]"),
    (
        r"maxmatching(petersengraph())",
        r"[[1, 2], [3, 4], [5, 10], [6, 8], [7, 9]]",
    ),
    (
        r"maxmatching(completebipartite(3, 3))",
        r"[[1, 4], [2, 5], [3, 6]]",
    ),
    (r"maxmatching([])", r"[]"),
    (r"maxmatching([[1]])", r"[]"),
    (r"maxclique([[1,2],[2,3],[3,1],[3,4]])", r"[1, 2, 3]"),
    (r"maxclique([[1,2],[3,4],[5]])", r"[1, 2]"),
    (
        r"maxclique([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[a, b, c]",
    ),
    (r"maxclique([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"[1, 2, 3]"),
    (r"maxclique([a -> b, b -> c, a -> c, d -> a])", r"[a, b, c]"),
    (r"maxclique([[1,2],[1,2],[2,2],[2,3]])", r"[1, 2]"),
    (r"maxclique(cyclegraph(5))", r"[1, 2]"),
    (r"maxclique(petersengraph())", r"[1, 2]"),
    (r"maxclique(completebipartite(3, 3))", r"[1, 4]"),
    (r"maxclique([])", r"[]"),
    (r"maxclique([[1]])", r"[1]"),
    (r"cliquenumber([[1,2],[2,3],[3,1],[3,4]])", r"3"),
    (r"cliquenumber([[1,2],[3,4],[5]])", r"2"),
    (r"cliquenumber([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"3"),
    (r"cliquenumber([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"3"),
    (r"cliquenumber([a -> b, b -> c, a -> c, d -> a])", r"3"),
    (r"cliquenumber([[1,2],[1,2],[2,2],[2,3]])", r"2"),
    (r"cliquenumber(cyclegraph(5))", r"2"),
    (r"cliquenumber(petersengraph())", r"2"),
    (r"cliquenumber(completebipartite(3, 3))", r"2"),
    (r"cliquenumber([])", r"0"),
    (r"cliquenumber([[1]])", r"1"),
    (r"maxindependentset([[1,2],[2,3],[3,1],[3,4]])", r"[1, 4]"),
    (r"maxindependentset([[1,2],[3,4],[5]])", r"[1, 3, 5]"),
    (
        r"maxindependentset([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[a, d]",
    ),
    (
        r"maxindependentset([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1, 4]",
    ),
    (
        r"maxindependentset([a -> b, b -> c, a -> c, d -> a])",
        r"[b, d]",
    ),
    (r"maxindependentset([[1,2],[1,2],[2,2],[2,3]])", r"[1, 3]"),
    (r"maxindependentset(cyclegraph(5))", r"[1, 3]"),
    (r"maxindependentset(petersengraph())", r"[3, 5, 6, 7]"),
    (r"maxindependentset(completebipartite(3, 3))", r"[1, 2, 3]"),
    (r"maxindependentset([])", r"[]"),
    (r"maxindependentset([[1]])", r"[1]"),
    (r"independencenumber([[1,2],[2,3],[3,1],[3,4]])", r"2"),
    (r"independencenumber([[1,2],[3,4],[5]])", r"3"),
    (
        r"independencenumber([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"2",
    ),
    (
        r"independencenumber([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"2",
    ),
    (
        r"independencenumber([a -> b, b -> c, a -> c, d -> a])",
        r"2",
    ),
    (r"independencenumber([[1,2],[1,2],[2,2],[2,3]])", r"2"),
    (r"independencenumber(cyclegraph(5))", r"2"),
    (r"independencenumber(petersengraph())", r"4"),
    (r"independencenumber(completebipartite(3, 3))", r"3"),
    (r"independencenumber([])", r"0"),
    (r"independencenumber([[1]])", r"1"),
    (r"vertexcover([[1,2],[2,3],[3,1],[3,4]])", r"[2, 3]"),
    (r"vertexcover([[1,2],[3,4],[5]])", r"[2, 4]"),
    (r"vertexcover([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])", r"[b, c]"),
    (r"vertexcover([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])", r"[2, 3]"),
    (r"vertexcover([a -> b, b -> c, a -> c, d -> a])", r"[a, c]"),
    (r"vertexcover([[1,2],[1,2],[2,2],[2,3]])", r"[2]"),
    (r"vertexcover(cyclegraph(5))", r"[2, 4, 5]"),
    (r"vertexcover(petersengraph())", r"[1, 2, 4, 8, 9, 10]"),
    (r"vertexcover(completebipartite(3, 3))", r"[4, 5, 6]"),
    (r"vertexcover([])", r"[]"),
    (r"vertexcover([[1]])", r"[]"),
    (
        r"complementgraph([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 4], [2, 4], [3]]",
    ),
    (
        r"complementgraph([[1,2],[3,4],[5]])",
        r"[[1, 3], [1, 4], [1, 5], [2, 3], [2, 4], [2, 5], [3, 5], [4, 5]]",
    ),
    (
        r"complementgraph([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[a, d], [b, d], [c]]",
    ),
    (
        r"complementgraph([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1 -> 3, 1 -> 4, 2 -> 1, 2 -> 4, 3 -> 2, 4 -> 1, 4 -> 2, 4 -> 3]",
    ),
    (
        r"complementgraph([a -> b, b -> c, a -> c, d -> a])",
        r"[a -> d, b -> a, b -> d, c -> a, c -> b, c -> d, d -> b, d -> c]",
    ),
    (
        r"complementgraph([[1,2],[1,2],[2,2],[2,3]])",
        r"[[1, 3], [2]]",
    ),
    (
        r"complementgraph(cyclegraph(5))",
        r"[[1, 3], [1, 4], [2, 4], [2, 5], [3, 5]]",
    ),
    (
        r"complementgraph(petersengraph())",
        r"[[1, 3], [1, 4], [1, 7], [1, 8], [1, 9], [1, 10], [2, 4], [2, 5], [2, 6], [2, 8], [2, 9], [2, 10], [3, 5], [3, 6], [3, 7], [3, 9], [3, 10], [4, 6], [4, 7], [4, 8], [4, 10], [5, 6], [5, 7], [5, 8], [5, 9], [6, 7], [6, 10], [7, 8], [8, 9], [9, 10]]",
    ),
    (
        r"complementgraph(completebipartite(3, 3))",
        r"[[1, 2], [1, 3], [2, 3], [4, 5], [4, 6], [5, 6]]",
    ),
    (r"complementgraph([])", r"[]"),
    (r"complementgraph([[1]])", r"[[1]]"),
    (
        r"linegraph([[1,2],[2,3],[3,1],[3,4]])",
        r"[[1, 2], [1, 3], [2, 3], [2, 4], [3, 4]]",
    ),
    (r"linegraph([[1,2],[3,4],[5]])", r"[[1], [2]]"),
    (
        r"linegraph([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[[1, 2], [1, 3], [2, 3], [2, 4], [3, 4]]",
    ),
    (
        r"linegraph([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"error: linegraph is for graphs without directions",
    ),
    (
        r"linegraph([a -> b, b -> c, a -> c, d -> a])",
        r"error: linegraph is for graphs without directions",
    ),
    (
        r"linegraph([[1,2],[1,2],[2,2],[2,3]])",
        r"[[1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4]]",
    ),
    (
        r"linegraph(cyclegraph(5))",
        r"[[1, 2], [1, 5], [2, 3], [3, 4], [4, 5]]",
    ),
    (
        r"linegraph(petersengraph())",
        r"[[1, 2], [1, 5], [1, 6], [1, 7], [2, 3], [2, 7], [2, 8], [3, 4], [3, 8], [3, 9], [4, 5], [4, 9], [4, 10], [5, 6], [5, 10], [6, 11], [6, 14], [7, 12], [7, 15], [8, 11], [8, 13], [9, 12], [9, 14], [10, 13], [10, 15], [11, 13], [11, 14], [12, 14], [12, 15], [13, 15]]",
    ),
    (
        r"linegraph(completebipartite(3, 3))",
        r"[[1, 2], [1, 3], [1, 4], [1, 7], [2, 3], [2, 5], [2, 8], [3, 6], [3, 9], [4, 5], [4, 6], [4, 7], [5, 6], [5, 8], [6, 9], [7, 8], [7, 9], [8, 9]]",
    ),
    (r"linegraph([])", r"[]"),
    (r"linegraph([[1]])", r"[]"),
    (
        r"pagerank([[1,2],[2,3],[3,1],[3,4]])",
        r"[770/3131, 770/3131, 4593/12524, 1771/12524]",
    ),
    (
        r"pagerank([[1,2],[3,4],[5]])",
        r"[20/83, 20/83, 20/83, 20/83, 3/83]",
    ),
    (
        r"pagerank([[a,b,7],[b,c,1],[a,c,3],[c,d,2]])",
        r"[770/3131, 770/3131, 4593/12524, 1771/12524]",
    ),
    (
        r"pagerank([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4])",
        r"[1429/6685, 1769/6685, 294/955, 1429/6685]",
    ),
    (
        r"pagerank([a -> b, b -> c, a -> c, d -> a])",
        r"[29600/127053, 28580/127053, 52873/127053, 16000/127053]",
    ),
    (
        r"pagerank([[1,2],[1,2],[2,2],[2,3]])",
        r"[743/2620, 72/131, 437/2620]",
    ),
    (r"pagerank(cyclegraph(5))", r"[1/5, 1/5, 1/5, 1/5, 1/5]"),
    (
        r"pagerank(petersengraph())",
        r"[1/10, 1/10, 1/10, 1/10, 1/10, 1/10, 1/10, 1/10, 1/10, 1/10]",
    ),
    (
        r"pagerank(completebipartite(3, 3))",
        r"[1/6, 1/6, 1/6, 1/6, 1/6, 1/6]",
    ),
    (r"pagerank([])", r"[]"),
    (r"pagerank([[1]])", r"[1]"),
    (
        r"completegraph(4)",
        r"[[1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4]]",
    ),
    (
        r"cyclegraph(5)",
        r"[[1, 2], [2, 3], [3, 4], [4, 5], [5, 1]]",
    ),
    (r"pathgraph(4)", r"[[1, 2], [2, 3], [3, 4]]"),
    (r"stargraph(4)", r"[[1, 2], [1, 3], [1, 4]]"),
    (
        r"wheelgraph(5)",
        r"[[1, 2], [1, 3], [1, 4], [1, 5], [2, 3], [3, 4], [4, 5], [5, 2]]",
    ),
    (
        r"gridgraph(2, 3)",
        r"[[1, 2], [1, 4], [2, 3], [2, 5], [3, 6], [4, 5], [5, 6]]",
    ),
    (
        r"hypercubegraph(3)",
        r"[[0, 1], [0, 2], [0, 4], [1, 3], [1, 5], [2, 3], [2, 6], [3, 7], [4, 5], [4, 6], [5, 7], [6, 7]]",
    ),
    (
        r"petersengraph()",
        r"[[1, 2], [2, 3], [3, 4], [4, 5], [5, 1], [1, 6], [2, 7], [3, 8], [4, 9], [5, 10], [6, 8], [7, 9], [8, 10], [9, 6], [10, 7]]",
    ),
    (
        r"completebipartite(2, 3)",
        r"[[1, 3], [1, 4], [1, 5], [2, 3], [2, 4], [2, 5]]",
    ),
    (r"completegraph(1)", r"[[1]]"),
    (r"pathgraph(1)", r"[[1]]"),
    (r"stargraph(1)", r"[[1]]"),
    (
        r"adjacencygraph([[0, 1, 1], [1, 0, 0], [1, 0, 0]])",
        r"[[1, 2], [1, 3]]",
    ),
    (r"adjacencygraph([[0, 1], [0, 0]])", r"[1 -> 2]"),
    (r"adjacencygraph([[0, 2], [2, 0]])", r"[[1, 2], [1, 2]]"),
    (r"adjacencygraph([[0, 5/2], [5/2, 0]])", r"[[1, 2, 5/2]]"),
    (r"adjacencygraph([[0, 0], [0, 0]])", r"[[1], [2]]"),
    (
        r"adjacencymatrix(adjacencygraph([[0, 1, 0], [0, 0, 1], [1, 0, 0]]))",
        r"[[0, 1, 0], [0, 0, 1], [1, 0, 0]]",
    ),
    (r"vertexdegree([[1,2],[2,3],[3,1],[3,4]], 3)", r"3"),
    (r"vertexdegree([[1,2],[1,2],[2,2],[2,3]], 2)", r"5"),
    (r"neighbors([[1,2],[2,3],[3,1],[3,4]], 3)", r"[1, 2, 4]"),
    (r"neighbors([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4], 3)", r"[1, 4]"),
    (r"neighbors(wheelgraph(6), 1)", r"[2, 3, 4, 5, 6]"),
    (
        r"shortestpath([[1,2],[2,3],[3,4],[1,4],[4,5]], 1, 5)",
        r"[1, 4, 5]",
    ),
    (
        r"shortestpath([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a, b)",
        r"[a, c, b]",
    ),
    (
        r"shortestpath([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a, d)",
        r"[a, c, d]",
    ),
    (
        r"shortestpath([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4], 4, 1)",
        r"[]",
    ),
    (
        r"shortestpath([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 4], 1, 4)",
        r"[1, 2, 3, 4]",
    ),
    (r"shortestpath([[1,2],[3,4]], 1, 3)", r"[]"),
    (r"shortestpath(gridgraph(3, 3), 1, 9)", r"[1, 2, 3, 6, 9]"),
    (r"shortestpath(petersengraph(), 1, 8)", r"[1, 6, 8]"),
    (
        r"graphdistance([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a, b)",
        r"4",
    ),
    (
        r"graphdistance([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a, d)",
        r"5",
    ),
    (r"graphdistance([[1,2],[3,4]], 1, 3)", r"inf"),
    (
        r"graphdistance([[1 -> 2, -1], [2 -> 3, 2], [1 -> 3, 4]], 1, 3)",
        r"1",
    ),
    (
        r"graphdistance([[a,b,sqrt(2)],[b,c,pi],[a,c,5]], a, c)",
        r"pi + sqrt(2)|  ~ 4.5558062159628882872643321074892",
    ),
    (
        r"graphdistance([[a,b,1/2],[b,c,1/3],[a,c,1]], a, c)",
        r"5/6|  ~ 0.83333333333333333333333333333333",
    ),
    (r"graphdistance(hypercubegraph(4), 0, 15)", r"4"),
    (r"eccentricity(pathgraph(5), 1)", r"4"),
    (r"eccentricity(pathgraph(5), 3)", r"2"),
    (r"eccentricity([[1,2],[3,4]], 1)", r"inf"),
    (r"eccentricity([[a,b,7],[b,c,1],[a,c,3],[c,d,2]], a)", r"5"),
    (r"diameter(gridgraph(4, 4))", r"6"),
    (r"diameter(hypercubegraph(4))", r"4"),
    (r"graphcenter(pathgraph(5))", r"[3]"),
    (r"graphcenter(pathgraph(4))", r"[2, 3]"),
    (r"graphradius(stargraph(6))", r"1"),
    (r"girth(completegraph(4))", r"3"),
    (r"girth(hypercubegraph(3))", r"4"),
    (r"girth(pathgraph(5))", r"inf"),
    (r"spanningtreecount(completegraph(5))", r"125"),
    (r"spanningtreecount(gridgraph(3, 3))", r"192"),
    (r"spanningtreecount(cyclegraph(7))", r"7"),
    (
        r"spanningtree(completegraph(4))",
        r"[[1, 2], [1, 3], [1, 4]]",
    ),
    (
        r"spanningtree([[a,b,sqrt(2)],[b,c,pi],[a,c,5]])",
        r"[[a, b, sqrt(2)], [b, c, pi]]",
    ),
    (
        r"chromaticpolynomial(completegraph(4), k)",
        r"k^4 - 6k^3 + 11k^2 - 6k",
    ),
    (r"chromaticpolynomial(pathgraph(4), q)", r"q*(q - 1)^3"),
    (
        r"chromaticpolynomial(hypercubegraph(3))",
        r"x^8 - 12x^7 + 66x^6 - 214x^5 + 441x^4 - 572x^3 + 423x^2 - 133x",
    ),
    (
        r"chromaticpolynomial(gridgraph(3, 3))",
        r"x^9 - 12x^8 + 66x^7 - 216x^6 + 459x^5 - 648x^4 + 594x^3 - 323x^2 + 79x",
    ),
    (
        r"chromaticpolynomial(wheelgraph(6))",
        r"x*(x - 1)*(x - 2)*(x - 3)*(x^2 - 4x + 5)",
    ),
    (r"chromaticnumber(completegraph(7))", r"7"),
    (r"chromaticnumber(gridgraph(4, 4))", r"2"),
    (r"chromaticnumber(wheelgraph(6))", r"4"),
    (r"chromaticnumber(wheelgraph(7))", r"3"),
    (
        r"graphcoloring(hypercubegraph(3))",
        r"[[0, 3, 5, 6], [1, 2, 4, 7]]",
    ),
    (
        r"graphcoloring(wheelgraph(6))",
        r"[[1], [2, 4], [3, 5], [6]]",
    ),
    (
        r"hamiltoniancycle(hypercubegraph(3))",
        r"[0, 1, 3, 2, 6, 7, 5, 4, 0]",
    ),
    (
        r"hamiltoniancycle(gridgraph(3, 4))",
        r"[1, 2, 3, 4, 8, 12, 11, 7, 6, 10, 9, 5, 1]",
    ),
    (r"hamiltoniancycle(gridgraph(3, 3))", r"[]"),
    (r"hamiltoniancycle(completegraph(4))", r"[1, 2, 3, 4, 1]"),
    (
        r"hamiltoniancycle([1 -> 2, 2 -> 3, 3 -> 1])",
        r"[1, 2, 3, 1]",
    ),
    (
        r"hamiltonianpath(gridgraph(3, 3))",
        r"[1, 2, 3, 6, 5, 4, 7, 8, 9]",
    ),
    (r"hamiltonianpath(stargraph(4))", r"[]"),
    (
        r"eulerianpath([[1,2],[2,3],[3,1],[3,4],[4,5],[5,3]])",
        r"[1, 2, 3, 4, 5, 3, 1]",
    ),
    (
        r"eulerianpath(completegraph(5))",
        r"[1, 2, 3, 1, 4, 2, 5, 3, 4, 5, 1]",
    ),
    (r"eulerianpath(completegraph(4))", r"[]"),
    (r"eulerianpath([1 -> 2, 2 -> 3, 3 -> 1])", r"[1, 2, 3, 1]"),
    (r"eulerianpath([1 -> 2, 2 -> 3])", r"[1, 2, 3]"),
    (r"iseulerian(completegraph(5))", r"true"),
    (r"iseulerian(completegraph(4))", r"false"),
    (
        r"maxflow([[s -> a, 3], [s -> b, 2], [a -> b, 1], [a -> t, 2], [b -> t, 3]], s, t)",
        r"5",
    ),
    (
        r"maxflow([[s -> a, 3], [s -> b, 2], [a -> b, 1], [a -> t, 2], [b -> t, 3]], t, s)",
        r"0",
    ),
    (
        r"maxflow([[1,2,3/2],[2,3,1/2],[1,3,2]], 1, 3)",
        r"5/2|  ~ 2.5",
    ),
    (r"maxflow(completegraph(5), 1, 5)", r"4"),
    (r"maxflow(petersengraph(), 1, 8)", r"3"),
    (r"maxmatching(completegraph(5))", r"[[1, 2], [3, 4]]"),
    (
        r"maxmatching(gridgraph(3, 3))",
        r"[[1, 2], [3, 6], [4, 5], [7, 8]]",
    ),
    (
        r"maxmatching([[1,2],[2,3],[3,1],[3,4],[4,5],[5,6],[6,4]])",
        r"[[1, 2], [3, 4], [5, 6]]",
    ),
    (r"maxclique(wheelgraph(6))", r"[1, 2, 3]"),
    (r"maxclique(complementgraph(cyclegraph(6)))", r"[1, 3, 5]"),
    (r"maxindependentset(gridgraph(3, 3))", r"[1, 3, 5, 7, 9]"),
    (r"vertexcover(gridgraph(3, 3))", r"[2, 4, 6, 8]"),
    (r"vertexcover(stargraph(6))", r"[1]"),
    (
        r"complementgraph(pathgraph(4))",
        r"[[1, 3], [1, 4], [2, 4]]",
    ),
    (
        r"complementgraph([1 -> 2, 2 -> 3])",
        r"[1 -> 3, 2 -> 1, 3 -> 1, 3 -> 2]",
    ),
    (r"linegraph(stargraph(4))", r"[[1, 2], [1, 3], [2, 3]]"),
    (
        r"linegraph(cyclegraph(4))",
        r"[[1, 2], [1, 4], [2, 3], [3, 4]]",
    ),
    (
        r"subgraph(completegraph(5), [1, 2, 3])",
        r"[[1, 2], [1, 3], [2, 3]]",
    ),
    (r"subgraph([[1,2],[2,3],[3,4]], [1, 3])", r"[[1], [3]]"),
    (
        r"subgraph([[a,b,7],[b,c,1],[a,c,3]], [a, c])",
        r"[[a, c, 3]]",
    ),
    (
        r"isisomorphic(cyclegraph(5), complementgraph(cyclegraph(5)))",
        r"true",
    ),
    (
        r"isisomorphic(petersengraph(), complementgraph(linegraph(completegraph(5))))",
        r"true",
    ),
    (
        r"isisomorphic(cyclegraph(6), completebipartite(3, 3))",
        r"false",
    ),
    (r"isisomorphic(hypercubegraph(2), cyclegraph(4))", r"true"),
    (r"isisomorphic(pathgraph(4), stargraph(4))", r"false"),
    (
        r"isisomorphic([1 -> 2, 2 -> 3], [a -> b, c -> b])",
        r"false",
    ),
    (r"isisomorphic([1 -> 2, 2 -> 3], [x -> y, y -> z])", r"true"),
    (
        r"pagerank([1 -> 2, 2 -> 3, 3 -> 1, 3 -> 2], 1/2)",
        r"[10/39, 5/13, 14/39]",
    ),
    (r"pagerank(cyclegraph(4))", r"[1/4, 1/4, 1/4, 1/4]"),
    (
        r"pagerank(stargraph(4))",
        r"[71/148, 77/444, 77/444, 77/444]",
    ),
    (
        r"eigenvalues(adjacencymatrix(cyclegraph(4)))",
        r"[-2, 0, 0, 2]",
    ),
    (r"det(laplacianmatrix(completegraph(4)))", r"0"),
    (r"eigenvalues(laplacianmatrix(pathgraph(3)))", r"[0, 1, 3]"),
    (r"vertexlist([[b,a],[c,a],[x+1,a]])", r"[b, a, c, x + 1]"),
    (r"vertexlist([[2,1],[pi,1],[1/2,e]])", r"[1/2, 1, 2, e, pi]"),
    (r"vertexlist([[c,a],[b,a]])", r"[a, b, c]"),
    (
        r"degrees(5)",
        r"error: a graph is a list of edges: [[1, 2], [2, 3]], [1 -> 2, 2 -> 3], with weights [[1, 2, 5], [2 -> 3, 4]], a vertex alone as [4]",
    ),
    (
        r"degrees([1,2])",
        r"error: a graph is a list of edges: [[1, 2], [2, 3]], [1 -> 2, 2 -> 3], with weights [[1, 2, 5], [2 -> 3, 4]], a vertex alone as [4]",
    ),
    (
        r"degrees([[1,2,3,4]])",
        r"error: a graph is a list of edges: [[1, 2], [2, 3]], [1 -> 2, 2 -> 3], with weights [[1, 2, 5], [2 -> 3, 4]], a vertex alone as [4]",
    ),
    (
        r"degrees([[[1],2]])",
        r"error: [1] cannot be a vertex: a graph is a list of edges: [[1, 2], [2, 3]], [1 -> 2, 2 -> 3], with weights [[1, 2, 5], [2 -> 3, 4]], a vertex alone as [4]",
    ),
    (
        r"shortestpath([[1,2]], 1, 9)",
        r"error: 9 is not a vertex of the graph",
    ),
    (
        r"graphdistance([[1,2,-1],[2,3,2]], 1, 3)",
        r"error: the graph has a way around with a weight below zero: there is no shortest path",
    ),
    (
        r"shortestpath([[1,2,w]], 1, 2)",
        r"error: the weight w is not a real number",
    ),
    (
        r"maxflow([[1,2]], 1, 1)",
        r"error: maxflow(graph, s, t) takes two different vertices",
    ),
    (
        r"maxflow([[a -> b, x]], a, b)",
        r"error: the capacity x is not a rational number from 0 on",
    ),
    (
        r"maxflow([[a -> b, -1]], a, b)",
        r"error: the capacity -1 is not a rational number from 0 on",
    ),
    (
        r"topologicalsort([[1,2]])",
        r"error: topologicalsort is for graphs with directions: [1 -> 2, 2 -> 3]",
    ),
    (
        r"topologicalsort([1 -> 2, 2 -> 1])",
        r"error: topologicalsort: the graph has a way around",
    ),
    (
        r"spanningtree([1 -> 2])",
        r"error: spanningtree is for graphs without directions",
    ),
    (
        r"spanningtreecount([1 -> 2])",
        r"error: spanningtreecount is for graphs without directions",
    ),
    (
        r"girth([1 -> 2])",
        r"error: girth is for graphs without directions",
    ),
    (
        r"maxmatching([1 -> 2])",
        r"error: maxmatching is for graphs without directions",
    ),
    (
        r"linegraph([1 -> 2])",
        r"error: linegraph is for graphs without directions",
    ),
    (
        r"chromaticpolynomial(gridgraph(3, 6))",
        r"error: the chromatic polynomial is found for graphs of up to 16 vertices",
    ),
    (
        r"graphcoloring([[1,1]])",
        r"error: a graph with a loop has no coloring",
    ),
    (r"chromaticpolynomial([[1,1]])", r"0"),
    (
        r"cyclegraph(2)",
        r"error: cyclegraph takes a whole number from 3 to 2000",
    ),
    (
        r"completegraph(0)",
        r"error: completegraph takes a whole number from 1 to 2000",
    ),
    (
        r"completegraph(x)",
        r"error: completegraph takes a whole number from 1 to 2000",
    ),
    (
        r"hypercubegraph(11)",
        r"error: hypercubegraph takes a dimension up to 10",
    ),
    (
        r"petersengraph(3)",
        r"error: petersengraph takes 0 argument(s), 1 given",
    ),
    (
        r"neighbors([[1,2]])",
        r"error: neighbors takes 2 argument(s), 1 given",
    ),
    (
        r"subgraph([[1,2]], 3)",
        r"error: subgraph(graph, [vertices])",
    ),
    (
        r"subgraph([[1,2]], [3])",
        r"error: 3 is not a vertex of the graph",
    ),
    (
        r"vertexdegree([[1,2]], 7)",
        r"error: 7 is not a vertex of the graph",
    ),
];

const PLOTS: &[(&str, &str)] = &[
    (
        r"plot(sin(x), x, 0, 2pi, 40, 8)",
        r" 1 │     ⢀⡠⠖⠚⠉⠉⠙⠲⢤⡀|   │   ⢀⡴⠋        ⠉⠲⣄|   │ ⢀⡴⠋            ⠈⠣⡀|   │⣠⠋                ⠙⢦⡀| 0 │⠁───────────────────⠳⣄────────────────⣠⠋|   │                     ⠈⢢⡀            ⣠⠞⠁|   │                       ⠙⠦⣀        ⣠⠞⠁|-1 │                         ⠈⠓⠦⣄⣀⣀⡤⠴⠊⠁|   └────────────────────────────────────────|    0                                    2pi",
    ),
    (
        r"plot(sin(x), x, 0, 2pi)",
        r" 1 │           ⣀⡤⠒⠋⠉⠉⠙⠒⠦⣄|   │         ⡠⠞⠁        ⠈⠙⢦⡀|   │       ⣠⠎              ⠙⢦|   │     ⢀⠞⠁                 ⠱⣄|   │    ⡰⠋                    ⠈⢦⡀|   │  ⢀⡜⠁                       ⠳⡄|   │ ⢠⠎                          ⠘⢆|   │⡰⠃                            ⠈⢧⡀| 0 │⠁───────────────────────────────⢳⡀────────────────────────────⢠⠎|   │                                 ⠱⡄                          ⡰⠃|   │                                  ⠘⢦                       ⢀⡜⠁|   │                                   ⠈⠳⡀                    ⣠⠎|   │                                     ⠙⢆                 ⢀⡴⠁|   │                                       ⠳⣄              ⡰⠋|   │                                        ⠈⠳⣄⡀        ⢀⡴⠊|-1 │                                           ⠙⠲⠤⣄⣀⣀⣠⠤⠚⠉|   └────────────────────────────────────────────────────────────────|    0                                                            2pi",
    ),
    (
        r"plot([sin(x), cos(x)], x, 0, 2pi, ascii, 40, 8)",
        r" 1 |++++   *******                      ++++|   |   ++++      ****                ++++|   |  *** +++       ***            +++|   |***     +++       ***        +++| 0 |*---------++--------**------++-------***|   |            ++       ***  ++       ***|   |              +++      +++*      ***|-1 |                ++++++++  *******|   +----------------------------------------|    0                                    2pi|* sin(x)   + cos(x)",
    ),
    (
        r"plot([sin(x), cos(x)], x, 0, 2pi, 40, 8)",
        r" 1 │⠉⠙⠒⢤⡀⢀⡠⠖⠚⠉⠉⠙⠲⢤⡀                    ⢀⡤⠒⠋⠉|   │   ⢀⡼⠻⣄       ⠉⠲⣄                ⣠⠞⠁|   │ ⢀⡴⠋  ⠈⠳⡄       ⠈⠣⡀            ⢠⠞⠁|   │⣠⠋      ⠈⢦⡀       ⠙⢦⡀        ⢀⡴⠁| 0 │⠁─────────⠱⣄────────⠳⣄──────⣠⠎────────⣠⠋|   │           ⠈⠳⡀       ⠈⢢⡀  ⢀⠞⠁       ⣠⠞⠁|   │             ⠈⠢⣄       ⠙⣦⣔⠁       ⣠⠞⠁|-1 │               ⠈⠓⠦⢄⣀⣀⡠⠴⠚⠁⠈⠓⠦⣄⣀⣀⡤⠴⠊⠁|   └────────────────────────────────────────|    0                                    2pi",
    ),
    (
        r"plot(tan(x), x, -3, 3, [-5, 5], 40, 8)",
        r" 5 │        ⡇           │       ⢰⠁|   │       ⡸            │       ⡞|   │     ⣀⠜⠁            │     ⣠⠞|   │⣀⡤⠤⠖⠋⠁              ⣀⣀⠤⠴⠒⠋| 0 │──────────────⣠⠤⠖⠒⠉⠉⠁─────────────⢀⣠⠴⠒⠚⠉|   │            ⡴⠋      │           ⢀⡔⠉|   │           ⡼        │           ⡎|-5 │          ⢀⠇        │          ⢸|   └────────────────────────────────────────|    -3                                     3",
    ),
    (
        r"plot(tan(x), x, -3, 3, 40, 8)",
        r" 12.79 │         ⡇          │        ⢸|       │        ⢸           │        ⡸|       │       ⢀⡞           │       ⢠⠇|       │⣀⣀⣀⣠⠤⠤⠖⠋            ⣀⣀⣀⣀⡠⠤⠴⠚⠁|     0 │───────────⢀⡤⠖⠒⠊⠉⠉⠉⠉⠁───────────⣠⠴⠒⠒⠋⠉⠉⠉|       │          ⢰⠃        │          ⡼⠁|       │          ⡎         │          ⡇|-12.79 │          ⡇         │         ⢸|       └────────────────────────────────────────|        -3                                     3",
    ),
    (
        r"plot(1/x, x, -2, 2, 40, 8)",
        r" 9.231 │                    │⡇|       │                    │⢱|       │                    │ ⠳⣄⡀|       │                    │   ⠉⠑⠒⠒⠦⠤⠤⠤⠤⠤⠤⠤⢤⣀⣀⣀|     0 │⠉⠉⠉⠓⠒⠒⠒⠒⠒⠒⠒⠲⠤⠤⢄⣀────┼───────────────────|       │               ⠈⠙⢦  │|       │                  ⢇ │|-9.231 │                  ⢸ │|       └────────────────────────────────────────|        -2                                     2",
    ),
    (
        r"plot(x^2, x, -2, 2, [0, 2], 40, 8)",
        r"2 │     ⠈⢧             │            ⡼⠁|  │      ⠈⢧            │           ⡼⠁|  │       ⠈⢧           │          ⡼⠁|  │         ⢣⡀         │        ⢀⡜|  │          ⠳⡄        │       ⢠⠞|  │           ⠘⢦⡀      │     ⢀⡴⠃|  │             ⠙⢦⡀    │   ⢀⡴⠋|0 │               ⠉⠒⠤⣄⣀⣀⣠⠤⠒⠉|  └────────────────────────────────────────|   -2                                     2",
    ),
    (
        r"plot(x^2, x, -2, 2, 40, 8, ascii)",
        r"4 |*                   |                  *|  | **                 |                **|  |  ***               |              ***|  |    ***             |            ***|  |      ***           |          ***|  |        ***         |        ***|  |          *****     |    *****|0 |              ************|  +----------------------------------------|   -2                                     2",
    ),
    (
        r"plot(sqrt(x), x, -1, 4, 40, 8)",
        r"2 │        │                        ⣀⣠⠤⠴⠒⠚⠉|  │        │                 ⣀⣠⠤⠴⠒⠋⠉|  │        │           ⢀⣠⠤⠖⠚⠉⠁|  │        │       ⣀⡤⠖⠊⠉|  │        │   ⢀⡤⠖⠋⠁|  │        │ ⡠⠞⠁|  │        ⢠⠞|0 │        ⡏|  └────────────────────────────────────────|   -1                                     4",
    ),
    (
        r"plot(floor(x), x, 0, 5, 40, 8)",
        r"5 │                                       ⢸|  │                               ⢠⠤⠤⠤⠤⠤⠤⠤⠼|  │                               ⢸|  │                       ⢸⠉⠉⠉⠉⠉⠉⠉⠉|  │                ⣀⣀⣀⣀⣀⣀⣀⣸|  │               ⢰⠁|  │        ⡖⠒⠒⠒⠒⠒⠒⠚|0 │⣀⣀⣀⣀⣀⣀⣀⣀⡇|  └────────────────────────────────────────|   0                                      5",
    ),
    (
        r"plot(3, x, 0, 1, 30, 5)",
        r"4.5 │|    │|    │⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤|    │|1.5 │|    └──────────────────────────────|     0                            1",
    ),
    (
        r"plot(exp(x), x, 0, 5, 40, 8)",
        r"148.4 │                                      ⢀⡞|      │                                     ⢀⠞|      │                                    ⡰⠋|      │                                  ⣠⠞⠁|      │                                ⣠⠞⠁|      │                            ⢀⣠⠔⠋|      │                      ⢀⣀⣠⠤⠖⠚⠉|    1 │⣀⣀⣀⣀⣀⣀⣀⣀⣀⣠⠤⠤⠤⠤⠤⠤⠴⠒⠒⠒⠋⠉⠉|      └────────────────────────────────────────|       0                                      5",
    ),
    (
        r"plot(ln(x), x, 0, 5, 40, 8)",
        r" 1.609 │                    ⢀⣀⣀⣀⣀⠤⠤⠤⠤⠤⠖⠒⠒⠒⠒⠒⠋⠉⠉⠉|       │          ⣀⣀⠤⠤⠖⠒⠒⠋⠉⠉⠉|     0 │────⢀⣠⠤⠒⠋⠉⠁─────────────────────────────|       │  ⣠⠖⠋|       │ ⡰⠁|       │⢰⠃|       │⣸|-4.159 │⡇|       └────────────────────────────────────────|        0                                      5",
    ),
    (
        r"plot(abs(x), x, -1, 1, 20, 4)",
        r"1 │⠑⢦⡀       │      ⢀⡴⠊|  │  ⠈⠳⢄⡀    │   ⢀⡠⠞⠁|  │     ⠙⠦⣀  │ ⣀⠴⠋|0 │       ⠈⠓⢤⡴⠚⠁|  └────────────────────|   -1                 1",
    ),
    (
        r"plot(gamma(x), x, -3, 4, 60, 12)",
        r" 13.49 │         ⡇      ⢸        │⡇|       │         ⡇      ⡎        │⢱|       │         ⡇      ⡇        │⢸|       │         ⢱     ⡸         │⠈⡇                             ⢀⣠⠖|       │         ⠈⠧⣀⣀⣀⠴⠃         │ ⠘⢦⡀                     ⢀⣀⣠⠤⠔⠚⠉|     0 │─────────────────────────┼───⠉⠙⠒⠒⠒⠲⠤⠤⠤⠤⠤⠤⠤⠖⠒⠒⠒⠒⠒⠚⠉⠉⠉────────|       │⢀⠔⠒⠒⠒⠒⢤⡀                 │|       │⢸      ⢱           ⣠⠴⠒⠦⣄ │|       │⡞      ⠈⡆         ⢰⠁   ⠘⡆│|       │⡇       ⡇         ⡏     ⢱│|       │⡇       ⡇         ⡇     ⢸│|-13.88 │        ⡇        ⢸       ⡇|       └────────────────────────────────────────────────────────────|        -3                                                         4",
    ),
    (
        r"plot(besselj(0, x), x, 0, 20, 60, 8)",
        r"      1 │⠉⠳⣄|        │  ⠈⢦|        │   ⠈⢧|        │    ⠈⢧              ⢀⡀|        │     ⠈⢆          ⢀⡴⠋⠉⠉⠙⠲⣄            ⣠⠤⠒⠲⠤⣄             ⣀⡤⠤⠤|      0 │──────⠈⢆────────⡰⠋──────⠈⠳⣄───────⢀⡴⠋──────⠙⢦⡀───────⣠⠔⠋⠁───|        │       ⠈⢧     ⢀⠞⠁         ⠈⠳⢤⣀ ⣀⡤⠞⠁          ⠈⠓⠦⠤⠤⠤⠖⠋|-0.4028 │         ⠑⢤⣀⣠⠔⠃               ⠉⠁|        └────────────────────────────────────────────────────────────|         0                                                         20",
    ),
    (
        r"plot(x, x, -1, 1, 8, 2)",
        r" 1 │    ⣀⡤⠖⠋|-1 │⣠⠴⠚⠉⠁───|   └────────|    -1     1",
    ),
    (
        r"plot(re(exp(i*x)), x, 0, 2pi, 40, 8)",
        r" 1 │⠉⠙⠒⢤⡀                              ⢀⡤⠒⠋⠉|   │    ⠈⠳⣄                          ⣠⠞⠁|   │      ⠈⠳⡄                      ⢠⠞⠁|   │        ⠈⢦⡀                  ⢀⡴⠁| 0 │──────────⠱⣄────────────────⣠⠎──────────|   │           ⠈⠳⡀            ⢀⠞⠁|   │             ⠈⠢⣄        ⣠⠔⠁|-1 │               ⠈⠓⠦⢄⣀⣀⡠⠴⠚⠁|   └────────────────────────────────────────|    0                                    2pi",
    ),
    (
        r"plot(sin(x), x, 0, 2pi, 400, 4)",
        r" 1 │                                    ⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣠⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠖⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠚⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠑⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⢤⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⡀|   │⣀⣀⣀⣀⣀⣀⣀⣀⣠⠤⠤⠤⠤⠤⠤⠤⠤⠖⠒⠒⠒⠒⠒⠒⠒⠒⠋⠉⠉⠉⠉⠉⠉⠉⠉⠉                                                                                                                                ⠉⠉⠉⠉⠉⠉⠉⠉⠉⠙⠒⠒⠒⠒⠒⠒⠒⠒⠲⠤⠤⠤⠤⠤⠤⠤⠤⢄⣀⣀⣀⣀⣀⣀⣀⣀⡀| 0 │⠁───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────⠉⠉⠉⠉⠉⠉⠉⠉⠑⠒⠒⠒⠒⠒⠒⠒⠒⠦⠤⠤⠤⠤⠤⠤⠤⠤⣄⣀⣀⣀⣀⣀⣀⣀⣀⣀────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────⣀⣀⣀⣀⣀⣀⣀⣀⣀⣠⠤⠤⠤⠤⠤⠤⠤⠤⠴⠒⠒⠒⠒⠒⠒⠒⠒⠋⠉⠉⠉⠉⠉⠉⠉⠉|-1 │                                                                                                                                                                                                                                           ⠈⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠓⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⢄⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⡤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠴⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠋⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉|   └────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────|    0                                                                                                                                                                                                                                                                                                                                                                                                            2pi",
    ),
    (
        r"plot(sin(x), t, 0, 1, 20, 4)",
        r"error: sin(x) has no real value from 0 to 1",
    ),
    (
        r"parametricplot([cos(t), sin(t)], t, 0, 2pi, 40, 10)",
        r" 1.05 │               ⣀⡤⠴⠒⠒⠒⠒⠦⢤⣀|      │            ⢀⡴⠋⠁    │   ⠈⠙⢦⡀|      │           ⢠⠏       │      ⠹⡄|      │          ⢠⠇        │       ⠸⡄|      │          ⢸         │        ⡇|    0 │──────────⢸─────────┼────────⡇──────────|      │          ⠘⡆        │       ⢰⠃|      │           ⠘⣆       │      ⣰⠃|      │            ⠈⠳⣄⡀    │   ⢀⣠⠞⠁|-1.05 │               ⠉⠓⠲⠤⠤⠤⠤⠖⠚⠉|      └────────────────────────────────────────|       -2.1                                 2.1",
    ),
    (
        r"parametricplot([sin(2t), sin(3t)], t, 0, 2pi, 40, 10, ascii)",
        r" 1.05 |           ***   ******   ***|      |          ** *****  | ***** **|      |          *  ****   |  ****  *|      |          ****   ** |**   ****|      |           **     ****     **|    0 |-----------**-----****-----**-----------|      |          ****   ** |**   ****|      |          *  ****   |  ****  *|      |          ** *****  | ***** **|-1.05 |           ***   ******   ***|      +----------------------------------------|       -2.1                                 2.1",
    ),
    (
        r"parametricplot([t, t^2], t, -1, 1, 30, 8)",
        r" 1.06 │⠠⡄             │            ⢠⠄|      │ ⠙⡄            │           ⢠⠋|      │  ⠘⣆           │          ⣰⠃|      │   ⠈⢦⡀         │        ⢀⡴⠁|      │     ⠳⣄        │       ⣠⠞|      │      ⠈⠳⣀      │     ⣀⠞⠁|      │        ⠈⠳⢄⡀   │  ⢀⡠⠞⠁|-0.06 │───────────⠉⠙⠒⠒⠒⠒⠋⠉───────────|      └──────────────────────────────|       -1.05                     1.05",
    ),
    (
        r"polarplot(cos(3t), t, 0, pi, 40, 10)",
        r" 0.9241 │           ⢠⠖⠲⢤⡀ │|        │           ⢸   ⠱⡄│|        │            ⢧⡀  ⢹⡀|        │             ⠱⣄  ⣇|        │              ⠈⠑⢦⣸⣠⠴⠒⠋⠉⠉⠉⠉⠉⠲⡄|      0 │──────────────⢀⡠⠞⢹⠙⠲⠤⣄⣀⣀⣀⣀⣀⠴⠃───────────|        │             ⡰⠋  ⡏|        │            ⡞⠁  ⣸⠁|        │           ⢸   ⡰⠃│|-0.9241 │           ⠘⠦⠴⠚⠁ │|        └────────────────────────────────────────|         -1.629                             2.067",
    ),
    (
        r"polarplot(1 + cos(t), t, 0, 2pi, 40, 10)",
        r" 1.364 │             │⢀⣠⠤⠒⠒⠒⠒⠦⢤⣀|       │            ⢀⡔⠋        ⠈⠓⢆|       │           ⢀⡏│            ⠳⡀|       │           ⢸ │             ⢱|       │            ⢧⣀             ⠈⡇|     0 │────────────⡞⠉─────────────⢀⡇───────────|       │           ⢸ │             ⡸|       │           ⠈⣇│            ⡴⠁|       │            ⠈⠣⣄        ⢀⡤⠎|-1.364 │             │⠈⠙⠒⠤⠤⠤⠤⠖⠚⠉|       └────────────────────────────────────────|        -1.853                             3.603",
    ),
    (
        r"polarplot(t, t, 0, 10, 40, 10, ascii)",
        r" 8.251 |                 ********|       |             *****    | ****|       |           ***        |     **|       |          **          |      **|       |         **       *****       **|     0 |---------*-------**---*-------**--------|       |        **       *    |       *|       |         *       **   |     **|       |         **        **********|-5.774 |          *           ||       +----------------------------------------|        -15.58                             12.47",
    ),
    (
        r"listplot([1, 4, 9, 16, 25, 36], 40, 8)",
        r"36 │                                       ⠙|   │|   │                               ⢴⠄|   │|   │                       ⢴⠄|   │                ⡀|   │        ⡀      ⠈⠋| 1 │⣄      ⠈⠋|   └────────────────────────────────────────|    1                                      6",
    ),
    (
        r"listplot([[0,0],[1,1],[2,4],[3,9]], 30, 6, ascii)",
        r"9 |                             *|  ||  ||  |                   *|  |          *|0 |*|  +------------------------------|   0                            3",
    ),
    (
        r"listplot([3, 1, 4, 1, 5, 9, 2, 6], 40, 6)",
        r"9 │                           ⠈⠋|  │|  │                      ⢀                ⠺|  │           ⢴⠄         ⠙⠁|  │⠗                                 ⡀|1 │     ⣠⡀         ⢀⣄               ⠈⠋|  └────────────────────────────────────────|   1                                      8",
    ),
    (
        r"listplot([5], 20, 4)",
        r"7.5 │|    │          ⡀|    │         ⠈⠋|2.5 │|    └────────────────────|     0                  2",
    ),
    (
        r"histogram([1,2,2,3,3,3,4,4,4,4,5,5,5,6,6,7])",
        r"  1 .. 2.2 │████████████████████████████████████ 3|2.2 .. 3.4 │████████████████████████████████████ 3|3.4 .. 4.6 │████████████████████████████████████████████████ 4|4.6 .. 5.8 │████████████████████████████████████ 3|5.8 .. 7   │████████████████████████████████████ 3",
    ),
    (
        r"histogram([1,2,2,3,3,3,4,4,4,4,5,5,5,6,6,7], 3, ascii)",
        r"1 .. 3 |##################### 3|3 .. 5 |################################################ 7|5 .. 7 |########################################## 6",
    ),
    (
        r"histogram([2,2,2])",
        r"2 .. 3 │████████████████████████████████████████████████ 3|3 .. 4 │ 0|4 .. 5 │ 0",
    ),
    (
        r"histogram([0.5, 1.5, 2.25, 2.75, 3, 9, 9.5, 10], 4)",
        r"  0.5 .. 2.875 │████████████████████████████████████████████████ 4|2.875 .. 5.25  │████████████ 1| 5.25 .. 7.625 │ 0|7.625 .. 10    │████████████████████████████████████ 3",
    ),
    (
        r"histogram(range(1, 101), 10)",
        r" 1 .. 11  │████████████████████████████████████████████ 10|11 .. 21  │████████████████████████████████████████████ 10|21 .. 31  │████████████████████████████████████████████ 10|31 .. 41  │████████████████████████████████████████████ 10|41 .. 51  │████████████████████████████████████████████ 10|51 .. 61  │████████████████████████████████████████████ 10|61 .. 71  │████████████████████████████████████████████ 10|71 .. 81  │████████████████████████████████████████████ 10|81 .. 91  │████████████████████████████████████████████ 10|91 .. 101 │████████████████████████████████████████████████ 11",
    ),
    (
        r"implicitplot(x^2 + y^2 = 1, x, -1.5, 1.5, y, -1.5, 1.5)",
        r" 1.5 │                │|     │                │|     │            ⢀⣀⣀⣠⣄⣀⣀⡀|     │         ⣠⠴⠚⠉   │  ⠉⠓⠦⣄|     │       ⣠⠞⠁      │     ⠈⠳⣄|     │      ⣰⠃        │       ⠘⣆|     │     ⢰⠃         │        ⠘⡆|     │     ⣸          │         ⣇|   0 │─────⢹──────────┼─────────⡏─────|     │     ⠸⡄         │        ⢠⠇|     │      ⠹⡄        │       ⢠⠏|     │       ⠙⢦⡀      │     ⢀⡴⠋|     │         ⠙⠲⢤⣀   │  ⣀⡤⠖⠋|     │            ⠈⠉⠉⠙⠋⠉⠉⠁|     │                │|-1.5 │                │|     └────────────────────────────────|      -1.5                         1.5",
    ),
    (
        r"implicitplot(x^2 - y^2 = 1/4, x, -1.2, 1.2, y, -1.2, 1.2, 40, 10)",
        r" 1.2 │⢤⣀                  │                 ⣀⡤|     │ ⠈⠙⠲⣄⡀              │             ⢀⣠⠖⠋⠁|     │     ⠉⠳⢤⡀           │          ⢀⡤⠞⠉|     │        ⠉⠳⣄         │        ⣠⠞⠉|     │          ⠈⢳        │       ⡞⠁|   0 │──────────⢀⡼────────┼───────⢧⡀──────────|     │        ⣀⡴⠋         │        ⠙⢦⣀|     │     ⣀⡴⠚⠁           │          ⠈⠓⢦⣀|     │ ⢀⣠⠴⠋⠁              │             ⠈⠙⠦⣄⡀|-1.2 │⠚⠉                  │                 ⠉⠓|     └────────────────────────────────────────|      -1.2                                 1.2",
    ),
    (
        r"implicitplot(y = x^2, x, -2, 2, y, -1, 4, 30, 8, ascii)",
        r" 4 |**             |            **|   | **            |           **|   |  ***          |         ***|   |    ***        |       ***|   |      ***      |     ***|   |        ****** |******| 0 |-------------****-------------|-1 |               ||   +------------------------------|    -2                           2",
    ),
    (
        r"implicitplot(x*y - 1, x, -3, 3, y, -3, 3, 30, 8)",
        r" 3 │               │⢸⡀|   │               │ ⢳⡀|   │               │  ⠙⠦⣄⡀|   │               │     ⠉⠉⠓⠒⠒⠒⠲⠤⠤| 0 │⠒⠒⠦⠤⠤⠤⢤⣀⣀──────┼──────────────|   │        ⠈⠙⠲⣄   │|   │           ⠈⢧  │|-3 │            ⠈⡇ │|   └──────────────────────────────|    -3                           3",
    ),
    (
        r"plot(ln(x), x, -1, 0)",
        r"error: ln(x) has no real value from -1 to 0",
    ),
    (
        r"plot(x, x, 2, 1)",
        r"error: from 2 to 1 is no range of real numbers",
    ),
    (
        r"plot(x)",
        r"error: plot takes at least 4 argument(s), 1 given",
    ),
    (
        r"1 + plot(x, x, 0, 1)",
        r"error: plot must be a statement of its own",
    ),
    (
        r"plot(sin(x), x, 0, 1, foo)",
        r"error: foo is no option of a plot: plot(f, x, a, b), plot([f, g], x, a, b); after them in any order: [low, high] for the vertical range, a width and a height in characters, ascii",
    ),
    (
        r"plot(sin(x), x, 0, 1, 500)",
        r"error: 500 is no option of a plot: plot(f, x, a, b), plot([f, g], x, a, b); after them in any order: [low, high] for the vertical range, a width and a height in characters, ascii",
    ),
    (
        r"plot(sin(x), x, 0, 1, [2, 1])",
        r"error: the range [2, 1] needs two numbers, the lower one first",
    ),
    (r"plot(sin(x), 3, 0, 1)", r"error: 3 is not a variable"),
    (
        r"plot(sin(x), x, a, 1)",
        r"error: from a to 1 is no range of real numbers",
    ),
    (
        r"implicitplot(x^2 + y^2 = -1, x, -1, 1, y, -1, 1)",
        r"error: the two sides are equal nowhere in these ranges, as far as the picture shows",
    ),
    (
        r"listplot([[1,2],[2,x]])",
        r"error: [2, x] is no point of real numbers",
    ),
    (
        r"listplot([])",
        r"error: listplot([y1, y2, ...]) or listplot([[x1, y1], [x2, y2], ...])",
    ),
    (
        r"histogram([])",
        r"error: histogram([values]) or histogram([values], bins)",
    ),
    (
        r"histogram([1, 2], 0)",
        r"error: histogram([values]) or histogram([values], bins)",
    ),
    (
        r"parametricplot([t, t^2, t^3], t, 0, 1)",
        r"error: parametricplot: parametricplot([x(t), y(t)], t, a, b), polarplot(r(t), t, a, b)",
    ),
    (
        r"polarplot([1, 2], t, 0, 1)",
        r"error: polarplot: parametricplot([x(t), y(t)], t, a, b), polarplot(r(t), t, a, b)",
    ),
];

#[test]
fn quaternion_equations() {
    check_with(&["-q"], QUATERNION_EQUATIONS);
}

#[test]
fn quaternion_unknowns_and_forms() {
    check_with(&["-q"], QUATERNION_UNKNOWNS_AND_FORMS);
}

#[test]
fn quaternion_equations_with_options() {
    check_with(&["-q", "-n"], QUATERNION_EQUATIONS_WITH_OPTIONS);
}

#[test]
fn quaternion_equations_exact() {
    check_with(&["-q", "-e"], QUATERNION_EQUATIONS_EXACT);
}

#[test]
fn quaternion_equations_fifty_digits() {
    check_with(&["-q", "-d", "50"], QUATERNION_EQUATIONS_FIFTY_DIGITS);
}

#[test]
fn qsolve_in_other_domains() {
    check(QSOLVE_IN_OTHER_DOMAINS);
}

#[test]
fn equations_and_systems() {
    check(EQUATIONS_AND_SYSTEMS);
}

#[test]
fn equations_in_one_unknown() {
    check(EQUATIONS_IN_ONE_UNKNOWN);
}

#[test]
fn polynomials_with_complex_coefficients() {
    check(POLYNOMIALS_WITH_COMPLEX_COEFFICIENTS);
}

#[test]
fn solutions_and_assumptions() {
    check(SOLUTIONS_AND_ASSUMPTIONS);
}

#[test]
fn real_domain_additions() {
    check_with(&["-r"], REAL_DOMAIN_ADDITIONS);
}

#[test]
fn units_in_si() {
    check(UNITS_IN_SI);
}

#[test]
fn unit_conversions() {
    check(UNIT_CONVERSIONS);
}

#[test]
fn graphs() {
    check(GRAPHS);
}

#[test]
fn plots() {
    check(PLOTS);
}
