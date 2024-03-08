import { Icon, useLocale } from '@sd/interface-core';
import { useLibraryMutation, useZodForm } from '@sd/client';
import { CheckBox, Dialog, Tooltip, useDialog, UseDialogProps } from '@sd/ui';

interface Props extends UseDialogProps {
	indexedArgs?: {
		locationId: number;
		rescan?: () => void;
		pathIds: number[];
	};
	ephemeralArgs?: {
		paths: string[];
	};
	dirCount?: number;
	fileCount?: number;
}

function getWording(dirCount: number, fileCount: number) {
	let type = 'file';
	let prefix = 'a';

	if (dirCount == 1 && fileCount == 0) {
		type = 'directory';
		prefix = 'a';
	}

	if (dirCount > 1 && fileCount == 0) {
		type = 'directories';
		prefix = dirCount.toString();
	}

	if (fileCount > 1 && dirCount == 0) {
		type = 'files';
		prefix = fileCount.toString();
	}

	if (fileCount > 0 && dirCount > 0) {
		type = 'items';
		prefix = (fileCount + dirCount).toString();
	}

	return { type, prefix };
}

export default (props: Props) => {
	const { t } = useLocale();
	const deleteFile = useLibraryMutation('files.deleteFiles');
	const deleteEphemeralFile = useLibraryMutation('ephemeralFiles.deleteFiles');

	const form = useZodForm();
	const { dirCount = 0, fileCount = 0, indexedArgs, ephemeralArgs } = props;

	const { type, prefix } = getWording(dirCount, fileCount);

	const icon = type === 'file' || type === 'files' ? 'Document' : 'Folder';

	const description = t('delete_warning', { type });

	return (
		<Dialog
			form={form}
			onSubmit={form.handleSubmit(async () => {
				if (indexedArgs != undefined) {
					const { locationId, rescan, pathIds } = indexedArgs;
					await deleteFile.mutateAsync({
						location_id: locationId,
						file_path_ids: pathIds
					});

					rescan?.();
				}

				if (ephemeralArgs != undefined) {
					const { paths } = ephemeralArgs;
					await deleteEphemeralFile.mutateAsync(paths);
				}
			})}
			icon={<Icon theme="light" name={icon} size={28} />}
			dialog={useDialog(props)}
			title={t('delete_dialog_title', { prefix, type })}
			description={description}
			loading={deleteFile.isLoading}
			ctaLabel={t('delete')}
			ctaDanger
			className="w-[200px]"
		>
			<Tooltip label={t('coming_soon')}>
				<div className="flex items-center pt-2 opacity-50">
					<CheckBox disabled className="!mt-0" />
					<p className="text-sm text-ink-dull">
						Delete all matching {type.endsWith('s') ? type : type + 's'}
					</p>
				</div>
			</Tooltip>
		</Dialog>
	);
};