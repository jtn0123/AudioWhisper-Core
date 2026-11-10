// Dumps the Swift runtime's and Foundation's scalar sets the formatter relies on, as ranges.
import Foundation

func ranges(_ name: String, _ test: (Unicode.Scalar) -> Bool) {
    var out: [String] = []
    var start: UInt32?
    for value in UInt32(0)...0x110000 {
        let hit = value < 0x110000 && Unicode.Scalar(value).map(test) == true
        if hit, start == nil { start = value }
        if !hit, let first = start {
            out.append(String(first, radix: 16) + "-" + String(value - 1, radix: 16))
            start = nil
        }
    }
    print(name, out.joined(separator: " "))
}

ranges("numeric") { $0.properties.numericType != nil }
ranges("alphabetic") { $0.properties.isAlphabetic }
ranges("whitespaces") { CharacterSet.whitespaces.contains($0) }
ranges("newlines") { CharacterSet.newlines.contains($0) }
// Foundation's non-literal search won't match a needle that a following scalar combines with.
ranges("extend") { scalar in
    let text = "\u{E000}" + String(Character(scalar))
    return text.replacingOccurrences(of: "\u{E000}", with: "").unicodeScalars.first?.value == 0xE000
}
