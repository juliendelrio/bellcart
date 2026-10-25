decision-migrate = À migrer
decision-archive = À archiver
decision-leave = À laisser
decision-review = À examiner

plan-title = Plan pour { $root }
plan-settled = Décidé par les règles et les recettes
plan-settled-empty = { $count ->
    [one] et { $count } dossier vide, non listé
   *[other] et { $count } dossiers vides, non listés
}
plan-nothing-settled = Aucune règle ne s'applique ici.
plan-unreadable = { $count ->
    [one] { $count } dossier illisible.
   *[other] { $count } dossiers illisibles.
} Relancez en administrateur pour les inclure.
plan-to-review = À examiner, dossier par dossier
plan-applications = Applications trouvées : { $names }
source-recipe = Selon la recette { $name }
source-arbiter = Réponses de l'arbitre { $backend }
source-user = Votre décision
scan-progress = Analyse : { $folders } dossiers, { $size } parcourus
plan-files ={ $count ->
    [one] { $count } fichier
   *[other] { $count } fichiers
}

packs-title = { $name } { $version }
packs-rules = { $count ->
    [one] { $count } règle
   *[other] { $count } règles
}
packs-recipes = { $count ->
    [one] { $count } recette
   *[other] { $count } recettes
}
packs-questions = { $count ->
    [one] { $count } question d'arbitrage
   *[other] { $count } questions d'arbitrage
}

reason-regenerable-dependencies = Dépendances réinstallées par un gestionnaire de paquets
reason-build-output = Résultat de compilation, recréé à la prochaine compilation
reason-package-cache = Cache de gestionnaire de paquets, retéléchargé à la demande
reason-application-cache = Cache d'application, reconstruit automatiquement
reason-temporary-files = Fichiers temporaires
reason-system-managed = Géré par Windows
reason-keys-and-credentials = Clés et identifiants : petits, et pénibles à perdre
reason-cloud-sync-cache = Cache local d'un stockage en ligne, resynchronisé par son logiciel
reason-synced-synology-drive = Dossier synchronisé avec un NAS Synology : son contenu est aussi sur le NAS, sauf les fichiers en attente d'envoi et ceux que les règles de synchronisation excluent
reason-other-applications = Applications trop petites pour une carte chacune (moins de 100 Mo). Leurs noms sont listés ci-dessous
reason-tool-folder = Réglages et données d'un outil ou d'un éditeur, rangés dans un dossier caché : regardez ce qu'il contient, ou archivez-le dans le doute
reason-app-extensions = Extensions : celles de la marketplace se réinstallent à partir de la liste que la recette enregistre
reason-unknown-application = Dossier d'une application que Bellcart ne connaît pas : regardez ce qu'il contient, ou archivez-le dans le doute
reason-app-cloud-account = Ses données sont sur les serveurs du service : l'application les retrouve quand vous vous reconnectez
reason-app-sync-client = Logiciel de synchronisation et son état : il les reconstruit quand vous le reconnectez
reason-app-downloaded-tools = Outils téléchargés automatiquement, retéléchargés au besoin
reason-app-browser-profile = Profil de navigateur : la recette ou la synchronisation de votre compte ramènent favoris et réglages. Le reste est surtout des données de sites : l'archiver le garde sans le recopier
reason-app-editor-state = État de l'éditeur (historique local, espaces de travail) : la recette prend déjà les réglages. L'archiver garde le reste sans le recopier
reason-app-mail = Courriels et carnets d'adresses stockés sur ce PC : ils peuvent n'exister qu'ici
reason-app-settings = Réglages et données de l'application, généralement petits : migrez-les pour la retrouver comme vous l'avez laissée
reason-app-virtual-disks = Disques de machines virtuelles ou de conteneurs : ils peuvent contenir vos données (volumes, distributions Linux). Les archiver les garde sans les recopier sur le nouveau PC
reason-crash-reports = Rapports de plantage, utiles seulement aux développeurs de l'application
reason-installed-program = Programme installé, à réinstaller sur le nouveau PC
reason-update-download = Mise à jour déjà téléchargée, récupérée de nouveau au besoin

unit-small = assez petit pour être jugé d'un bloc
unit-homogeneous = surtout des fichiers .{ $extension } ({ $percent } %)
unit-homogeneous-no-extension = surtout des fichiers sans extension ({ $percent } %)
unit-depth-limit = dossier profond, jugé d'un bloc
unit-leaf = aucun sous-dossier assez gros pour être jugé à part
unit-rest = fichiers posés dans ce dossier et ses petits sous-dossiers

notebook-title = Carnet de migration pour { $root }
notebook-intro = Cochez chaque point au fur et à mesure. Les dossiers que le plan migre sont copiés avec le reste : ces points sont ce que les fichiers seuls n'emportent pas.
notebook-empty = Aucune application connue d'une recette n'a été trouvée sur ce profil.
notebook-before = Avant de quitter l'ancien PC
notebook-after = À l'arrivée sur le nouveau PC
notebook-copy = Emportez { $path }
notebook-copy-note = Emportez { $path } : { $note }
notebook-export-winget = Enregistrez la liste des applications installées avec winget (bellcart export peut le faire pour vous) : { $command }
notebook-export-vscode = Enregistrez la liste des extensions de Visual Studio Code (bellcart export peut le faire pour vous) : { $command }
notebook-export-registry = Enregistrez la clé de registre { $key } (bellcart export peut le faire pour vous) : { $command }
notebook-install = Installez-la avec winget : { $command }

export-intro = Bellcart ne lance chaque commande ci-dessous que si vous répondez oui. Les fichiers vont dans { $folder }.
export-none = Rien à exporter : aucune application trouvée sur ce profil n'a d'étape d'export.
export-winget = Applications installées avec winget
export-vscode = Extensions de Visual Studio Code
export-registry = Clé de registre { $key }
export-ask = La lancer ? [o/N]
export-skipped = Ignorée.
export-done = Enregistré : { $file }
export-failed = Échec : { $error }
export-missing = { $program } est introuvable sur ce PC.

arbitrate-none = Rien à soumettre à un arbitre : aucun dossier n'est à examiner.
arbitrate-preview = Voici la requête exacte pour le premier dossier. Les autres ne diffèrent que par leur résumé.
arbitrate-confirm = Envoyer { $count } résumés de dossiers à { $url } ? [o/N]
arbitrate-nothing-sent = Rien n'a été envoyé.
arbitrate-failed = { $path } : { $error }
arbitrate-stopped = Arrêt après { $count } échecs d'affilée : le point d'accès répond-il ?
arbitrate-leave-to-confirm = l'arbitre propose de le laisser : à confirmer

label-prompt = m (migrer), a (archiver), l (laisser), s (sauter), q (quitter) ?
label-already = { $count } dossiers déjà étiquetés sont sautés.
label-done = { $count } nouvelles étiquettes dans { $file }.
eval-confirm = Envoyer { $count } résumés étiquetés à { $url } ? [o/N]
eval-title = { $backend } sur { $count } dossiers étiquetés : { $agreed } en accord
eval-label = { $label } : { $agreed } sur { $total } en accord
eval-lost-none = Aucun dossier à garder ne serait laissé derrière.
eval-lost = Dossiers à garder qui seraient laissés derrière : { $paths }
eval-leave-suggested = Laisser proposé, et retenu pour l'utilisateur, sur des dossiers à migrer : { $paths }
eval-failed = Sans réponse : { $paths }

ui-listening = Bellcart est ouvert dans votre navigateur : { $url } (Ctrl+C ici pour l'arrêter)
ui-not-a-folder = { $path } n'est pas un dossier.
ui-title = Bellcart
ui-step-choose = Choisir
ui-step-analyse = Analyser
ui-step-review = Examiner
ui-step-summary = Vérifier
ui-step-export = Exporter
ui-choose-title = Quel profil voulez-vous trier ?
ui-choose-help = Bellcart ne fait que lire. Il ne déplace, ne modifie et ne supprime rien sur ce PC, et rien n'en sort sans votre accord.
ui-choose-label = Dossier à analyser
ui-choose-start = Analyser
ui-resume-title = Reprendre là où vous en étiez ?
ui-resume-help-one = { $count } décision est enregistrée pour { $profile }, dans { $file }.
ui-resume-help-other = { $count } décisions sont enregistrées pour { $profile }, dans { $file }.
ui-resume = Reprendre
ui-start-over = Recommencer (les décisions enregistrées seront remplacées)
ui-resumed-one = { $count } décision reprise
ui-resumed-other = { $count } décisions reprises
ui-saved-to = Vos décisions sont enregistrées au fur et à mesure, dans { $file }.
ui-not-saved = Vos décisions ne sont pas enregistrées : le dossier d'export proposé est dans le profil.
ui-save-failed = La décision est prise mais n'a pas pu être enregistrée : { $error }
ui-scanning-title = Analyse en cours
ui-scanning-help = Bellcart liste les dossiers et leur taille. Il n'ouvre aucun fichier.
ui-failed = L'analyse a échoué : { $error }
ui-error = Un problème est survenu : { $error }
ui-carts = Chariots
ui-to-decide = À décider
ui-folder-one = { $count } dossier
ui-folder-other = { $count } dossiers
ui-file-one = { $count } fichier
ui-file-other = { $count } fichiers
ui-unreadable-one = { $count } dossier illisible
ui-unreadable-other = { $count } dossiers illisibles
ui-review-title = Examen
ui-review-help = Décidez dossier par dossier, les plus gros d'abord. Ce que les règles et les recettes ont déjà réglé est replié plus bas, et reste modifiable.
ui-applications = Applications trouvées : { $names }
ui-card-position = Dossier { $index } sur { $count }
ui-card-modified = modifié de { $oldest } à { $newest }
ui-card-kinds = Types de fichiers
ui-card-folders = Plus gros dossiers
ui-card-files = Plus gros fichiers
ui-card-settled = Déjà réglé à l'intérieur
ui-card-opinion = L'arbitre propose : { $decision }
ui-card-opinion-leave = L'arbitre propose de laisser ce dossier. Bellcart ne laisse jamais un dossier sur la seule parole d'un modèle : décidez vous-même.
ui-card-propose = Proposé : { $decision }.
ui-key-enter = Entrée
ui-no-extension = sans extension
ui-more = { $count } de plus
ui-none = Aucun
ui-action-migrate = Migrer
ui-action-archive = Archiver
ui-action-leave = Laisser
ui-action-later = Plus tard
ui-decided = { $path } : { $decision }
ui-queue-empty = Tout est décidé.
ui-queue-empty-help = Vous pouvez encore changer toute décision ci-dessous, puis exporter le plan et le carnet.
ui-go-export = Vérifier et exporter
ui-summary-title = Vérifiez avant d'exporter
ui-summary-help = Voici ce que diront le plan et le carnet. Rien n'est écrit avant l'export, et chaque décision peut encore être changée.
ui-summary-split = Répartition du profil
ui-summary-share = { $decision } : { $size }, { $percent } %
ui-summary-attention = Points à vérifier
ui-summary-all-clear = Rien à revoir : chaque dossier est décidé, et aucun n'est laissé par un choix de votre part.
ui-summary-undecided-one = { $count } dossier non décidé ({ $size }). Il reste « à examiner » dans le plan : rien ne sera fait pour lui.
ui-summary-undecided-other = { $count } dossiers non décidés ({ $size }). Ils restent « à examiner » dans le plan : rien ne sera fait pour eux.
ui-summary-left-by-you-one = { $count } dossier que vous avez choisi de laisser ({ $size }). Vérifiez que rien n'y existe seulement sur ce PC.
ui-summary-left-by-you-other = { $count } dossiers que vous avez choisi de laisser ({ $size }). Vérifiez que rien n'y existe seulement sur ce PC.
ui-summary-left-personal-one = { $count } dossier de vos dossiers personnels est laissé par une règle ({ $size }). Vérifiez que la raison convient.
ui-summary-left-personal-other = { $count } dossiers de vos dossiers personnels sont laissés par une règle ({ $size }). Vérifiez que la raison convient.
ui-summary-unreadable-one = { $count } dossier n'a pas pu être lu : il n'est pas dans le plan. Lancez Bellcart en administrateur pour l'inclure.
ui-summary-unreadable-other = { $count } dossiers n'ont pas pu être lus : ils ne sont pas dans le plan. Lancez Bellcart en administrateur pour les inclure.
ui-summary-largest = Les plus gros dossiers de chaque chariot
ui-summary-cart-empty = Rien dans ce chariot.
ui-summary-decide = Décider
ui-summary-change = Changer
ui-summary-applications = Le carnet dira quoi faire, avant le départ et à l'arrivée, pour : { $names }
ui-summary-confirm = C'est bon, passer à l'export
ui-summary-back = Retour à l'examen
ui-settled-title = Décisions réglées ({ $count })
ui-settled-help = Les règles, les recettes, l'arbitre et vous avez décidé de ceux-ci. Corrigez celles qui sont fausses.
ui-col-decision = Décision
ui-col-folder = Dossier
ui-col-size = Taille
ui-col-why = Pourquoi
ui-col-change = Changer
ui-change-for = Changer la décision pour { $path }
ui-arbiter-ask = Demander à l'arbitre
ui-arbiter-help = Un arbitre est configuré : { $url }. Rien n'est envoyé avant que vous l'ayez vu.
ui-arbiter-preview-title = Envoyer { $count } résumés de dossiers ?
ui-arbiter-preview-help = Voici la requête exacte pour le premier dossier, vers { $url } (modèle { $model }). Les autres ne diffèrent que par leur résumé. Noms : { $privacy }.
ui-privacy-full = envoyés tels quels
ui-privacy-redacted = masqués sous les dossiers personnels
ui-arbiter-send = Envoyer
ui-cancel = Annuler
ui-arbiter-progress = L'arbitre a répondu pour { $done } sur { $total }
ui-arbiter-failed = L'arbitre s'est arrêté : { $error }
ui-export-title = Exporter le plan et le carnet
ui-export-help = Choisissez un dossier hors du profil analysé, par exemple sur une clé USB. Bellcart y écrit le plan (plan.json), le carnet de migration (notebook.html et notebook.md) et vos décisions.
ui-export-inside = Ce dossier est dans le profil analysé ({ $profile }) : Bellcart n'y écrit jamais. Choisissez un dossier en dehors, par exemple sur une clé USB.
ui-export-label = Dossier d'export
ui-export-go = Écrire les fichiers
ui-export-done = Fichiers écrits
ui-export-next = Ouvrez notebook.html maintenant et suivez « Avant de quitter l'ancien PC » tant que ce PC est là. Emportez ensuite le dossier sur le nouveau PC.
ui-back = Retour
ui-quit = Fermer Bellcart
ui-closed = Bellcart est fermé. Vous pouvez fermer cet onglet.
ui-root = (tout le profil)
ui-crumb-up = Décider à partir de { $path }, avec tout son contenu
ui-scope-title = Décider pour tout { $path }
ui-scope-pending-one = { $count } dossier encore à décider dessous ({ $size })
ui-scope-pending-other = { $count } dossiers encore à décider dessous ({ $size })
ui-scope-changed-one = { $count } décision déjà prise dessous sera remplacée
ui-scope-changed-other = { $count } décisions déjà prises dessous seront remplacées
ui-scope-kept-one = { $count } dossier réglé par une règle ou une recette garde sa décision
ui-scope-kept-other = { $count } dossiers réglés par une règle ou une recette gardent leur décision
ui-scope-back = Revenir à { $path } seul
ui-decided-below-one = { $path } et son contenu : { $decision } ({ $count } changement)
ui-decided-below-other = { $path } et son contenu : { $decision } ({ $count } changements)
