# Aiome Avatar System Specification

This document defines the architectural standards for the Aiome Avatar system as of the v1.0 Launch. Following these standards ensures visual consistency and structural integrity across future development cycles.

## 1. State Management Architecture

The avatar state is managed via a dedicated React Context following the **Single Source of Truth** principle.

- **Location**: `apps/management-console/src/hooks/AvatarContext.tsx`
- **Key States**:
    - `character`: `'female' | 'male'` (Persistent in `localStorage`)
    - `proportion`: `'taller' | 'chibi'` (Persistent in `localStorage`)
- **Helper Method**: `getAssetPath(mode: 'lite' | 'vrm')`
    - Use this method exclusively to retrieve image paths. Do **not** hardcode paths in UI components.

## 2. Asset Catalog (v1.0 Launch Set)

All assets are located in `apps/management-console/public/avatar/`.

### Female Character (Primary)
| Mode | Chibi (SD) | Modern Taller (Balanced) |
| :--- | :--- | :--- |
| **Lite** | `aiome-chibi.png` | `aiome-lite-female-taller.png` |
| **VRM (Nobg)** | `aiome-chibi-nobg.png` | `aiome-main-female-taller.png` |

### Male Character (Variant)
| Mode | Chibi (SD) | Modern Taller (Balanced) |
| :--- | :--- | :--- |
| **Lite** | `aiome-lite-male-chibi.png` | `aiome-lite-male-taller.png` |
| **VRM (Nobg)** | `aiome-male-chibi-nobg.png` | `aiome-main-male-taller.png` |

## 3. Visual Standards & Proportions

To maintain the aesthetic "feel" of Aiome, new assets must adhere to these guidelines:

- **Chibi (SD)**: 2.0 to 2.5 heads tall. Cute, large eyes, simplified techwear.
- **Modern Taller**: 7.5 to 8.0 heads tall. Professional model proportions. Slender silhouette.
- **Techwear Aesthetic**: Blue cyber jackets, glowing cyan accents, and tactical/high-fashion elements are mandatory for the "Aiome Legacy" look.

## 4. Component Integration Rules

### Lite Mode Components
Components like `AiomeAvatar.tsx` should use `getAssetPath('lite')`. These assets typically include stylized backgrounds or UI flourishes baked into the image.

### VRM (Diorama) Components
Components like `DioramaView.tsx` (using `VrmRenderer`) should use `getAssetPath('vrm')`. These assets **must** have a transparent background or a pure black background that is keyed out by the renderer.

## 5. Future Evolution Guidelines

When adding a third character or a new style:
1.  **Update Types**: Add the new key to `AvatarCharacter` or `AvatarProportion` types in `AvatarContext.tsx`.
2.  **Update Map**: Register the new assets in the `AVATAR_ASSETS` constant in `AvatarContext.tsx`.
3.  **UI Exposure**: Add the corresponding selection buttons in `SettingsPage.tsx`.

---
*Authorized by Antigravity Core Integration Unit.*
