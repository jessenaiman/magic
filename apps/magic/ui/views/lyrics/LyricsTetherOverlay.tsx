import React from "react";

export interface TetherCoords {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  cx1: number;
  cy1: number;
  cx2: number;
  cy2: number;
}

export const LyricsTetherOverlay: React.FC<{ tetherCoords: TetherCoords | null }> = ({ tetherCoords }) => (
  <>
      {/* Musical-Note Curved Tether Linking Selected Moment Card -> Stage Word Box */}
      {tetherCoords && (
        <svg
          id="words-tether-svg"
          className="lyrics-tether-svg"
          aria-hidden="true"
        >
          <defs>
            <path
              id="words-tether-curve-def"
              d={`M ${tetherCoords.x1} ${tetherCoords.y1} C ${tetherCoords.cx1} ${tetherCoords.cy1}, ${tetherCoords.cx2} ${tetherCoords.cy2}, ${tetherCoords.x2} ${tetherCoords.y2}`}
            />
          </defs>
          <use
            href="#words-tether-curve-def"
            className="lyrics-tether-glow"
          />
          <use
            href="#words-tether-curve-def"
            className="lyrics-tether-stroke"
          />
          <text className="lyrics-tether-notes" dy="-6">
            <textPath href="#words-tether-curve-def" startOffset="28%">
              ♪ &nbsp;&nbsp;&nbsp; ♫ &nbsp;&nbsp;&nbsp; ♬
            </textPath>
          </text>
          <circle
            cx={tetherCoords.x2}
            cy={tetherCoords.y2}
            r={6.5}
            className="lyrics-tether-target-dot"
          />
        </svg>
      )}

  </>
);
