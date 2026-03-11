import React, { createContext, useContext, useState, useEffect } from 'react';

export type AvatarCharacter = 'female' | 'male';
export type AvatarProportion = 'taller' | 'chibi';
export type AvatarMode = 'vrm' | 'lite' | 'off';

/**
 * Avatar Asset Specification
 * This map serves as the single source of truth for the launch avatar assets.
 */
export const AVATAR_ASSETS = {
    female: {
        lite: {
            chibi: '/avatar/aiome-chibi.png',
            taller: '/avatar/aiome-lite-female-taller.png',
        },
        vrm: {
            chibi: '/avatar/aiome-chibi-nobg.png',
            taller: '/avatar/aiome-main-female-taller.png',
        }
    },
    male: {
        lite: {
            chibi: '/avatar/aiome-lite-male-chibi.png',
            taller: '/avatar/aiome-lite-male-taller.png',
        },
        vrm: {
            chibi: '/avatar/aiome-male-chibi-nobg.png',
            taller: '/avatar/aiome-main-male-taller.png',
        }
    }
} as const;

interface AvatarCharacterContextType {
    character: AvatarCharacter;
    setCharacter: (char: AvatarCharacter) => void;
    proportion: AvatarProportion;
    setProportion: (prop: AvatarProportion) => void;
    getAssetPath: (mode: 'lite' | 'vrm') => string;
}

const AvatarCharacterContext = createContext<AvatarCharacterContextType | undefined>(undefined);

export const AvatarCharacterProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [character, setCharacter] = useState<AvatarCharacter>(() => {
        const saved = localStorage.getItem('aiome_avatar_character');
        if (saved === 'female' || saved === 'male') {
            return saved;
        }
        return 'female';
    });

    const [proportion, setProportion] = useState<AvatarProportion>(() => {
        const saved = localStorage.getItem('aiome_avatar_proportion');
        if (saved === 'taller' || saved === 'chibi') {
            return saved;
        }
        return 'chibi';
    });

    useEffect(() => {
        localStorage.setItem('aiome_avatar_character', character);
    }, [character]);

    useEffect(() => {
        localStorage.setItem('aiome_avatar_proportion', proportion);
    }, [proportion]);

    // Central logic to retrieve asset paths
    const getAssetPath = (mode: 'lite' | 'vrm') => {
        return AVATAR_ASSETS[character][mode][proportion];
    };

    return (
        <AvatarCharacterContext.Provider value={{
            character,
            setCharacter,
            proportion,
            setProportion,
            getAssetPath
        }}>
            {children}
        </AvatarCharacterContext.Provider>
    );
};

export const useAvatarCharacter = () => {
    const context = useContext(AvatarCharacterContext);
    if (!context) {
        throw new Error('useAvatarCharacter must be used within an AvatarCharacterProvider');
    }
    return context;
};
